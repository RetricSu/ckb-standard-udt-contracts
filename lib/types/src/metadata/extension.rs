use alloc::vec::Vec;

#[cfg(all(not(feature = "std"), feature = "no-std"))]
use ckb_std::ckb_types::{packed::Script, prelude::*};
#[cfg(feature = "std")]
use ckb_types::{packed::Script, prelude::*};
use molecule::prelude::{Builder, Entity};

use crate::{error::Error, generated};

use super::config::MAX_EXTENSIONS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionType {
    DynamicLinking,
    Spawn,
}

impl TryFrom<u8> for ExtensionType {
    type Error = Error;

    fn try_from(extension_type: u8) -> Result<Self, Self::Error> {
        match extension_type {
            0 => Ok(Self::DynamicLinking),
            1 => Ok(Self::Spawn),
            _ => Err(Error::InvalidScriptLocation),
        }
    }
}

impl From<ExtensionType> for u8 {
    fn from(extension_type: ExtensionType) -> Self {
        match extension_type {
            ExtensionType::DynamicLinking => 0,
            ExtensionType::Spawn => 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Extension {
    pub extension_type: ExtensionType,
    pub script: Script,
}

pub(crate) fn unpack_extension_vec(
    raw: generated::metadata::ExtensionVec,
) -> Result<Vec<Extension>, Error> {
    if raw.len() > MAX_EXTENSIONS {
        return Err(Error::ExtensionsTooMany);
    }

    let extensions = raw
        .into_iter()
        .map(unpack_extension)
        .collect::<Result<Vec<_>, Error>>()?;
    validate_extensions(&extensions)?;
    Ok(extensions)
}

fn unpack_extension(raw: generated::metadata::Extension) -> Result<Extension, Error> {
    Ok(Extension {
        extension_type: ExtensionType::try_from(u8::from(raw.extension_type()))?,
        script: Script::from_slice(raw.script().as_slice())?,
    })
}

pub(crate) fn pack_extension_vec(
    extensions: &[Extension],
) -> Result<generated::metadata::ExtensionVec, Error> {
    validate_extensions(extensions)?;
    let mut builder = generated::metadata::ExtensionVec::new_builder();
    for extension in extensions {
        builder = builder.push(pack_extension(extension)?);
    }
    Ok(builder.build())
}

fn pack_extension(extension: &Extension) -> Result<generated::metadata::Extension, Error> {
    Ok(generated::metadata::Extension::new_builder()
        .extension_type(u8::from(extension.extension_type).into())
        .script(generated::blockchain::Script::from_slice(
            extension.script.as_slice(),
        )?)
        .build())
}

pub(crate) fn validate_extensions(extensions: &[Extension]) -> Result<(), Error> {
    if extensions.len() > MAX_EXTENSIONS {
        return Err(Error::ExtensionsTooMany);
    }

    let mut prev_key: Option<(u8, [u8; 32])> = None;
    for extension in extensions {
        let script_hash: [u8; 32] = extension.script.calc_script_hash().unpack();
        let key = (u8::from(extension.extension_type), script_hash);
        if let Some(prev) = prev_key {
            if key == prev {
                return Err(Error::ExtensionsDuplicated);
            }
            if key < prev {
                return Err(Error::ExtensionsNotSorted);
            }
        }
        prev_key = Some(key);
    }
    Ok(())
}
