use windows::core::HRESULT;

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("Windows Error")]
    Inner(#[from] windows::core::Error),
}

impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Self {
        Error::Inner(err)
    }
}
