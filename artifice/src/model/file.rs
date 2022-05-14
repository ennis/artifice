use crate::model::{node::Node, path::Path, share_group::ShareGroup, AttributeAny, Document, EditAction, Value};
use anyhow::Result;
use artifice::model::NodeEditProxy;
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
              parent  INTEGER)",
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
        "CREATE TABLE IF NOT EXISTS nodes \
                                    (obj_id INTEGER REFERENCES named_objects(id) ON DELETE CASCADE)",
        [],
    )?;

    conn.execute(
        // language=SQLITE-SQL
        "CREATE TABLE IF NOT EXISTS attributes \
                                    (obj_id      INTEGER REFERENCES named_objects(id) ON DELETE CASCADE, \
                                     type        TEXT NOT NULL, \
                                     connection  TEXT, \
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

///
pub trait DocumentBackend {
    /// Inserts a node.
    fn insert_node(&mut self, path: &Path) -> Result<i64>;

    /// Inserts a node attribute.
    fn insert_attribute(
        &mut self,
        path: &Path,
        ty: Atom,
        value: Option<Value>,
        connection: Option<Path>,
    ) -> Result<i64>;

    /// Sets the value of an attribute.
    fn set_attribute_value(&mut self, id: i64, value: Option<Value>) -> Result<()>;

    /// Sets the connection of an attribute.
    fn set_attribute_connection(&mut self, id: i64, connection: Option<Path>) -> Result<()>;

    /// Removes a node.
    fn remove_node(&mut self, id: i64) -> Result<()>;

    /// Removes an attribute.
    fn remove_attribute(&mut self, id: i64) -> Result<()>;
}

#[derive(Debug)]
pub struct DocumentDatabase {
    db: Connection,
}

impl DocumentDatabase {
    /// Inserts a row into the `named_objects` table (nodes & attributes).
    fn insert_named_object(&mut self, parent: Option<i64>, path: &Path) -> Result<i64> {
        let name = path.name().to_string();
        let path_str = path.to_string();
        // language=SQLITE-SQL
        self.db.execute(
            "INSERT INTO named_objects(name,path,parent) VALUES (?1,?2,?3)",
            params![name, path_str, parent],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    /// Inserts a node.
    pub(crate) fn insert_node(&mut self, parent: i64, path: &Path) -> Result<i64> {
        let id = self.insert_named_object(Some(parent), path)?;
        // language=SQLITE-SQL
        self.db.execute("INSERT INTO nodes(obj_id) VALUES (?1)", params![id])?;
        Ok(id)
    }

    /// Inserts a node attribute.
    pub(crate) fn insert_attribute(
        &mut self,
        parent: i64,
        path: &Path,
        ty: Atom,
        value: Option<Value>,
        connection: Option<Path>,
    ) -> Result<i64> {
        let id = self.insert_named_object(Some(parent), path)?;
        let value_json = value.as_ref().map(|v| serde_json::to_string(v).unwrap());
        let ty_str = ty.to_string();
        let conn_str = connection.as_ref().map(|c| c.to_string());
        // language=SQLITE-SQL
        self.db.execute(
            "INSERT INTO attributes(obj_id,type,value,connection) VALUES (?1,?2,?3,?4)",
            params![id, ty_str, value_json, conn_str],
        )?;
        Ok(id)
    }

    /// Sets the value of an attribute.
    pub(crate) fn set_attribute_value(&mut self, id: i64, value: Option<Value>) -> Result<()> {
        let value_json = value.as_ref().map(|v| serde_json::to_string(v).unwrap());
        // language=SQLITE-SQL
        self.db
            .execute("UPDATE attributes SET value=?1 WHERE id=?2", params![value_json, id])?;
        Ok(())
    }

    /// Sets the connection of an attribute.
    pub(crate) fn set_attribute_connection(&mut self, id: i64, connection: Option<Path>) -> Result<()> {
        let conn_str = connection.as_ref().map(|c| c.to_string());
        // language=SQLITE-SQL
        self.db
            .execute("UPDATE attributes SET connection=?1 WHERE id=?2", params![conn_str, id])?;
        Ok(())
    }

    /// Removes a node
    pub(crate) fn remove_node(&mut self, id: i64) -> Result<()> {
        // TODO remove child nodes
        // language=SQLITE-SQL
        self.db
            .execute("DELETE FROM named_objects WHERE rowid=?1", params![id])?;
        Ok(())
    }

    pub(crate) fn remove_attribute(&mut self, id: i64) -> Result<()> {
        // language=SQLITE-SQL
        self.db
            .execute("DELETE FROM attributes WHERE node_id=?1", params![id])?;
        Ok(())
    }

    /*/// Reads all the attributes of a node.
    pub(crate) fn read_attributes(&mut self, node: &mut Node) -> Result<()> {
        // language=SQLITE-SQL
        let mut stmt = self.db.prepare("SELECT a.obj_id, a.type, a.value, a.connection, no.path FROM attributes a INNER JOIN named_objects no ON a.obj_id = no.id WHERE no.parent = ?1 ORDER BY no.path")?;
        let mut attr_rows = stmt.query(params![node.id])?;

        while let Some(row) = attr_rows.next()? {
            let id: i64 = row.get(0)?;
            let ty: String = row.get(1)?;
            let value: Option<String> = row.get(2)?;
            let connection: Option<String> = row.get(3)?;
            let path: String = row.get(4)?;
            let path = Path::parse(&path);
            node.attributes.insert(
                path.name(),
                AttributeAny {
                    rev: 0,
                    id,
                    path,
                    ty: ty.into(),
                    value: None,
                    connection: connection.map(|c| Path::parse(&c)),
                    metadata: Default::default(),
                },
            );
        }
    }

    pub(crate) fn read_child_nodes(&mut self, node: &mut Node) -> Result<()> {
        let mut stmt = connection.prepare(
            "SELECT no.rowid, no.path FROM nodes n INNER JOIN named_objects no ON n.obj_id = no.id ORDER BY no.path",
        )?;

        let mut node_rows = stmt.query([])?;
        while let Some(row) = node_rows.next()? {
            let id: i64 = row.get(0)?;
            let path = match row.get_ref(1)? {
                ValueRef::Null => Path::root(),
                e @ ValueRef::Text(text) => Path::parse(e.as_str()?),
                _ => {
                    anyhow::bail!("invalid column type")
                }
            };
            nodes.push((path.to_string(), Node::new(id, path)));
        }
    }*/
}

/// Wrapper over the SQLite connection to a document.
#[derive(Debug)]
pub struct DocumentFile {
    /// Connection to the SQLite DB.
    db: DocumentDatabase,
    /// Revision number, incremented on every change.
    revision: usize,
    /// In-memory document model.
    document: Document,
}

impl DocumentFile {
    /// Opens a document from a sqlite database connection.
    pub fn open(connection: Connection) -> Result<DocumentFile> {
        // check for correct application ID
        if let Ok(ARTIFICE_APPLICATION_ID) = connection.pragma_query_value(None, "application_id", |v| v.get(0)) {
            // OK, app id matches, assume schema is in place
        } else {
            setup_schema(&connection)?;
        }

        // create initial document object
        let mut document = Document::new();

        // load nodes, ordering is important for later, when building the tree
        let mut nodes = Vec::new();

        {
            let mut stmt = connection.prepare("SELECT no.rowid, no.path FROM nodes n INNER JOIN named_objects no ON n.obj_id = no.id ORDER BY no.path")?;
            let mut node_rows = stmt.query([])?;

            while let Some(row) = node_rows.next()? {
                let id: i64 = row.get(0)?;
                let path = match row.get_ref(1)? {
                    ValueRef::Null => Path::root(),
                    e @ ValueRef::Text(text) => Path::parse(e.as_str()?),
                    _ => {
                        anyhow::bail!("invalid column type")
                    }
                };

                nodes.push((path.to_string(), Node::new(id, path)));
            }
        }

        // build hierarchy
        // FIXME this is not very efficient (we're cloning a lot)
        // this works because `nodes` is already ordered by path from the query
        for (_, n) in nodes {
            if n.path.is_root() {
                document.root = n;
            } else {
                let (parent_path, name) = n.path.split_last().unwrap();
                let mut parent = document.node_mut(&parent_path).unwrap();
                parent.children.insert(name, n);
            }
        }

        // load attributes
        {
            let mut stmt = connection.prepare("SELECT a.obj_id, a.type, a.value, a.connection, no.path FROM attributes a INNER JOIN named_objects no ON a.obj_id = no.id ORDER BY no.path")?;
            let mut attr_rows = stmt.query([])?;

            while let Some(row) = attr_rows.next()? {
                let id: i64 = row.get(0)?;
                let ty: String = row.get(1)?;
                let value_str: Option<String> = row.get(2)?;
                let value: Option<Value> = value_str
                    .as_ref()
                    .map(|v| serde_json::from_str(v).expect("invalid json"));
                let connection: Option<String> = row.get(3)?;
                let path: String = row.get(4)?;
                let path = Path::parse(&path);
                let parent = path.parent().unwrap();
                document.node_mut(&parent).unwrap().attributes.insert(
                    path.name(),
                    AttributeAny {
                        rev: 0,
                        id,
                        path,
                        ty: ty.into(),
                        value,
                        connection: connection.map(|c| Path::parse(&c)),
                        metadata: Default::default(),
                    },
                );
            }
        }

        Ok(DocumentFile {
            db: DocumentDatabase { db: connection },
            revision: 0,
            document,
        })
    }

    /// Performs editing on the document
    pub fn edit<R>(&mut self, f: impl FnOnce(&mut NodeEditProxy) -> R) -> R {
        self.document.edit(&mut self.db, f)
    }

    /// Returns the document revision number.
    pub fn revision(&self) -> usize {
        self.revision
    }

    /// Returns the document model root.
    pub fn document(&self) -> &Document {
        &self.document
    }
}
