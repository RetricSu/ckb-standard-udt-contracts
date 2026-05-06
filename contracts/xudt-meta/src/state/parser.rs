use standard_udt_types::metadata::XudtMeta;

use crate::error::Error;

pub(crate) fn parse_meta(data: &[u8]) -> Result<XudtMeta, Error> {
    XudtMeta::from_slice(data).map_err(Error::from)
}
