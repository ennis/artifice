use crate::model::{node::Node, path::ModelPath, share_group::ShareGroup, NamedObject};
use anyhow::Result;
use imbl::Vector;
use kyute::Data;
use parking_lot::{Mutex, MutexGuard};
use rusqlite::{params, types::ValueRef, Connection};
use std::sync::{Arc, Weak};

const ARTIFICE_APPLICATION_ID: i32 = 0x41525446;

/// Creates the database tables.
fn setup_schema(conn: &rusqlite::Connection) -> Result<()> {
    // named_objects: {obj_id} -> name, parent_obj_id      (name, parent must be unique)
    // share_groups: {share_id, obj_id}

    conn.execute(
        "CREATE TABLE IF NOT EXISTS named_objects \
             (id      INTEGER PRIMARY KEY, \
              name    TEXT NOT NULL, \
              path    TEXT UNIQUE NOT NULL, \
              parent  TEXT)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS share_groups \
                            (share_id     INTEGER,\
                             obj_id       INTEGER,\
                             PRIMARY KEY (share_id, obj_id))",
        [],
    )?;

    // insert root node
    conn.execute(
        "INSERT OR IGNORE INTO named_objects (name, path, parent) VALUES ('','',null)",
        [],
    );

    conn.pragma_update(None, "application_id", ARTIFICE_APPLICATION_ID)?;
    Ok(())
}

/// Root object of documents.
#[derive(Clone, Debug)]
pub struct DocumentModel {
    /// Document revision index
    pub revision: usize,
    /// Root node
    pub root: Node,
    /// Share groups
    pub share_groups: Vector<ShareGroup>,
}

// TODO data impls for imbl
impl Data for DocumentModel {
    fn same(&self, other: &Self) -> bool {
        self.revision.same(&other.revision)
            && self.root.same(&other.root)
            && self.share_groups.ptr_eq(&other.share_groups)
    }
}

impl DocumentModel {
    /// Finds the node with the given path.
    pub fn find_node(&self, path: &ModelPath) -> Option<&Node> {
        match path.split_last() {
            None => Some(&self.root),
            Some((prefix, last)) => {
                let parent = self.find_node(&prefix)?;
                parent.find_child(&last)
            }
        }
    }

    /// Finds the node with the given path and returns a mutable reference to it.
    pub fn find_node_mut(&mut self, path: &ModelPath) -> Option<&mut Node> {
        match path.split_last() {
            None => Some(&mut self.root),
            Some((prefix, last)) => {
                let parent = self.find_node_mut(&prefix)?;
                parent.find_child_mut(&last)
            }
        }
    }
}

/// Wrapper over the SQLite connection to a document.
#[derive(Debug)]
pub struct Document {
    connection: Connection,
    revision: usize,
    model: DocumentModel,
}

pub struct Edit<'a> {
    transaction: rusqlite::Transaction<'a>,
}

impl Document {
    /// Opens a document from a sqlite database connection.
    pub fn open(connection: Connection) -> Result<Document> {
        // check for correct application ID
        if let Ok(ARTIFICE_APPLICATION_ID) =
            connection.pragma_query_value(None, "application_id", |v| v.get(0))
        {
            // OK, app id matches, assume schema is in place
        } else {
            setup_schema(&connection)?;
        }

        // create initial document object
        let mut model = DocumentModel {
            revision: 0,
            root: Node {
                base: NamedObject {
                    id: 0,
                    path: ModelPath::root(),
                },
                children: Default::default(),
            },
            share_groups: Default::default(),
        };

        // load nodes, ordering is important for later, when building the tree
        let mut nodes = Vec::new();

        {
            let mut stmt =
                connection.prepare("SELECT rowid, path FROM named_objects ORDER BY path")?;
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
                    },
                ));
            }
        }

        // build hierarchy
        // FIXME this is not very efficient (we're cloning a lot)
        // this works because `nodes` is already ordered by path from the query
        for (_, n) in nodes.iter() {
            if n.base.path.is_root() {
                model.root = n.clone();
            } else {
                let mut parent = model.find_node_mut(&n.base.path.parent().unwrap()).unwrap();
                parent.add_child(n.clone());
            }
        }

        Ok(Document {
            connection,
            revision: 0,
            model,
        })
    }

    /// Returns the document revision number.
    pub fn revision(&self) -> usize {
        self.revision
    }

    /// Returns the document model root.
    pub fn model(&self) -> &DocumentModel {
        &self.model
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
        };
        parent.add_child(node.clone());
        Ok(node)
    }

    pub fn dump(&self) {
        println!("Document");
        self.model.root.dump(0);
    }
}
