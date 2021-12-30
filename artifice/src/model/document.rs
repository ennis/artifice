use crate::model::{
    network::Network, node::Node, path::ModelPath, share_group::ShareGroup, NamedObject,
};
use anyhow::Result;
use imbl::Vector;
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

/// Wrapper over the SQLite connection to a document.
#[derive(Debug)]
struct Document {
    connection: Connection,
    revision: usize,
    model: DocumentModel,
}

/// Root object of documents.
#[derive(Clone, Data)]
pub struct DocumentModel {
    /// Document revision index
    pub revision: usize,
    /// Root node
    pub root: Node,
    /// Share groups
    pub share_groups: Vector<ShareGroup>,
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
        let mut model = Arc::new(DocumentModel {
            revision: 0,
            root: Node {
                base: NamedObject {
                    document: Default::default(),
                    id: 0,
                    path: ModelPath::root(),
                },
                children: Default::default(),
            },
            share_groups: Default::default(),
        });

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
                        base: NamedObject {
                            document: Arc::downgrade(&model),
                            id,
                            path,
                        },
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
                let mut parent = document
                    .find_node_mut(&n.base.path.parent().unwrap())
                    .unwrap();
                parent.add_child(n.clone());
            }
        }

        Ok(document)
    }

    /*pub fn write(&self, conn: &rusqlite::Connection) -> Result<()> {
        // recursively write nodes
        self.root.write(conn)?;
        Ok(())
    }*/

    pub fn find_node(&self, path: &ModelPath) -> Option<&Node> {
        match path.split_last() {
            None => Some(&self.model.root),
            Some((prefix, last)) => {
                let parent = self.find_node(&prefix)?;
                parent.find_child(&last)
            }
        }
    }

    pub fn find_node_mut(&mut self, path: &ModelPath) -> Option<&mut Node> {
        match path.split_last() {
            None => Some(&mut self.model.root),
            Some((prefix, last)) => {
                let parent = self.find_node_mut(&prefix)?;
                parent.find_child_mut(&last)
            }
        }
    }

    /*fn insert_node(&mut self, node: Node) {
        // to reconstruct: sort nodes by path, lexicographically?
        let mut parent = self
            .find_node_mut(&node.base.path.parent().unwrap())
            .unwrap();
    }*/

    ///
    pub fn create_node(&mut self, conn: &rusqlite::Connection, path: ModelPath) -> Result<Node> {
        assert!(!path.is_root());

        // TODO check that parent exists before inserting in the DB

        // insert the node in the DB first, this will take care of ensuring that the path is unique
        let path_str = path.to_string();
        let name = path.name().to_string();

        conn.execute(
            "INSERT INTO named_objects (name, path, parent) VALUES (?1,?2,null)",
            params![name, path_str],
        )?;

        let id = conn.last_insert_rowid();

        let parent = self
            .find_node_mut(&path.parent().unwrap())
            .expect("parent node not found");
        let n = Node {
            base: NamedObject { id, path },
            children: Default::default(),
        };
        parent.add_child(n.clone());
        Ok(n)
    }

    pub fn dump(&self) {
        println!("Document");
        self.model.dump(0);
    }
}
