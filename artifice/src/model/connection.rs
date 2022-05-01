use crate::model::{attribute::Attribute, node::Node, path::ModelPath, share_group::ShareGroup, Document, NamedObject};
use anyhow::Result;
use imbl::Vector;
use kyute_common::{Atom, Data};
use parking_lot::{Mutex, MutexGuard};
use rusqlite::{params, types::ValueRef, Connection};
use std::sync::{Arc, Weak};

const ARTIFICE_APPLICATION_ID: i32 = 0x41525446;

/// Creates the database tables.
fn setup_schema(conn: &rusqlite::Connection) -> Result<()> {
    // named_objects: {obj_id} -> name, parent_obj_id      (name, parent must be unique)
    // share_groups: {share_id, obj_id, is_master}
    // attributes: {obj_id} -> type,value

    conn.execute(
        // language=SQLITE-SQL
        "CREATE TABLE IF NOT EXISTS named_objects \
             (id      INTEGER PRIMARY KEY, \
              name    TEXT NOT NULL, \
              path    TEXT UNIQUE NOT NULL, \
              parent  TEXT)",
        [],
    )?;

    conn.execute(
        // language=SQLITE-SQL
        "CREATE TABLE IF NOT EXISTS share_groups \
                            (share_id     INTEGER,\
                             obj_id       INTEGER,\
                             PRIMARY KEY (share_id, obj_id))",
        [],
    )?;

    conn.execute(
        // language=SQLITE-SQL
        "CREATE TABLE IF NOT EXISTS attributes \
                                    (obj_id INTEGER REFERENCES named_objects(id) ON DELETE CASCADE, \
                                     type   TEXT NOT NULL, \
                                     value)",
        [],
    )?;

    // insert root node
    conn.execute(
        // language=SQLITE-SQL
        "INSERT OR IGNORE INTO named_objects (name, path, parent) VALUES ('','',null)",
        [],
    )?;

    conn.pragma_update(None, "application_id", ARTIFICE_APPLICATION_ID)?;
    Ok(())
}

/// Wrapper over the SQLite connection to a document.
#[derive(Debug)]
pub struct DocumentConnection {
    /// Connection to the SQLite DB.
    connection: Connection,
    /// Revision number, incremented on every change.
    revision: usize,
    /// In-memory document model.
    document: Document,
}

pub struct Edit<'a> {
    transaction: rusqlite::Transaction<'a>,
}

impl DocumentConnection {
    /// Opens a document from a sqlite database connection.
    pub fn open(connection: Connection) -> Result<DocumentConnection> {
        // check for correct application ID
        if let Ok(ARTIFICE_APPLICATION_ID) = connection.pragma_query_value(None, "application_id", |v| v.get(0)) {
            // OK, app id matches, assume schema is in place
        } else {
            setup_schema(&connection)?;
        }

        // create initial document object
        let mut document = Document {
            revision: 0,
            root: Node {
                base: NamedObject {
                    id: 0,
                    path: ModelPath::root(),
                },
                children: Default::default(),
                attributes: Default::default(),
            },
            share_groups: Default::default(),
        };

        // load nodes, ordering is important for later, when building the tree
        let mut nodes = Vec::new();

        {
            let mut stmt = connection.prepare("SELECT rowid, path FROM named_objects ORDER BY path")?;
            let mut node_rows = stmt.query([])?;

            while let Some(row) = node_rows.next()? {
                let id: i64 = row.get(0)?;
                let path = match row.get_ref(1)? {
                    ValueRef::Null => ModelPath::root(),
                    e @ ValueRef::Text(text) => ModelPath::parse(e.as_str()?),
                    _ => {
                        anyhow::bail!("invalid column type")
                    }
                };

                nodes.push((
                    path.to_string(),
                    Node {
                        base: NamedObject { id, path },
                        children: Default::default(),
                        attributes: Default::default(),
                    },
                ));
            }
        }

        // build hierarchy
        // FIXME this is not very efficient (we're cloning a lot)
        // this works because `nodes` is already ordered by path from the query
        for (_, n) in nodes.iter() {
            if n.base.path.is_root() {
                document.root = n.clone();
            } else {
                let mut parent = document.find_node_mut(&n.base.path.parent().unwrap()).unwrap();
                parent.add_child(n.clone());
            }
        }

        Ok(DocumentConnection {
            connection,
            revision: 0,
            document,
        })
    }

    /// Returns the document revision number.
    pub fn revision(&self) -> usize {
        self.revision
    }

    /// Returns the document model root.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Creates a new node.
    pub fn create_node(&mut self, path: ModelPath) -> Result<Node> {
        // Can't create with root path.
        assert!(!path.is_root());

        // check that parent exists before inserting in the DB
        let parent = self
            .model
            .find_node_mut(&path.parent().unwrap())
            .expect("parent node not found");

        // insert the node in the DB, this will take care of ensuring that the path is unique
        let path_str = path.to_string();
        let name = path.name().to_string();
        self.connection.execute(
            "INSERT INTO named_objects (name, path, parent) VALUES (?1,?2,null)",
            params![name, path_str],
        )?;

        // get id
        let id = self.connection.last_insert_rowid();

        // increase revision
        self.revision += 1;

        // add the node object in the document model
        let node = Node {
            base: NamedObject { id, path },
            children: Default::default(),
            attributes: Default::default(),
        };
        parent.add_child(node.clone());
        Ok(node)
    }

    /// Adds an attribute to a node
    pub fn create_attribute(&mut self, parent_node_path: ModelPath, name: Atom, ty: Atom) -> Result<Attribute> {
        let parent_str = parent_node_path.to_string();
        let name_str = name.to_string();
        let path = parent_node_path.join(name.clone());
        let path_str = path.to_string();
        let parent = self.model.find_node_mut(&parent_node_path).expect("parent not found");

        //  create named object
        self.connection.execute(
            "INSERT INTO named_objects (name,path,parent) VALUES (?1,?2,?3)",
            params![name_str, path_str, parent_str],
        )?;
        let id = self.connection.last_insert_rowid();

        // create attribute object
        let type_str = ty.to_string();
        self.connection.execute(
            "INSERT INTO attributes (id,type,value) VALUES (?1,?2,?3)",
            params![id, type_str, ""],
        );

        // insert attribute in memory model
        let attr = Attribute {
            base: NamedObject { id, path },
            ty,
            value: Default::default(),
        };
        parent.attributes.insert(name, attr.clone());

        self.revision += 1;
        Ok(attr)
    }

    /// Deletes the specified named object.
    pub fn delete_object(&mut self, path: &ModelPath) -> Result<()> {
        assert!(!path.is_root(), "attempted to delete the root node");
        todo!()
    }

    /// Deletes the specified node.
    pub fn delete_node(&mut self, node: &Node) -> Result<()> {
        // Can't delete root node.
        assert!(!node.base.path.is_root(), "attempted to delete the root node");

        //let path_str = node.base.path.to_string();
        let id = node.base.id;
        self.connection
            .execute("DELETE FROM named_objects WHERE rowid=?1", params![id])?;
        self.revision += 1;
        let mut parent = self
            .model
            .find_node_mut(&node.base.path.parent().unwrap())
            .expect("could not find parent node");
        parent.children.remove(&node.base.name());
        Ok(())
    }

    /// Dumps the document to the standard output.
    pub fn dump(&self) {
        println!("--- Document ---");
        self.document.root.dump(0);
    }
}
