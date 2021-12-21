use crate::model::network::Network;

/// Document.
#[derive(Clone, Data)]
pub struct Document {

    /// Network being edited
    pub network: Network,

    /// Path to the file being edited, empty if not saved yet
    #[data(ignore)]
    pub current_file_info: Option<FileInfo>,
}
