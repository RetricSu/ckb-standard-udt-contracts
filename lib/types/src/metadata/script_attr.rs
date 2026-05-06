use alloc::vec::Vec;

#[cfg(all(not(feature = "std"), feature = "no-std"))]
use ckb_std::ckb_types::{packed::Script, prelude::*};
#[cfg(feature = "std")]
use ckb_types::{packed::Script, prelude::*};
use molecule::prelude::{Builder, Entity};

use crate::{error::Error, generated};

use super::config::MAX_EXTENSIONS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptLocation {
    InputLock,
    InputType,
    OutputType,
    DynamicLinking,
    Spawn,
}

impl TryFrom<u8> for ScriptLocation {
    type Error = Error;

    fn try_from(location: u8) -> Result<Self, Self::Error> {
        match location {
            0 => Ok(Self::InputLock),
            1 => Ok(Self::InputType),
            2 => Ok(Self::OutputType),
            3 => Ok(Self::DynamicLinking),
            4 => Ok(Self::Spawn),
            _ => Err(Error::InvalidScriptLocation),
        }
    }
}

impl From<ScriptLocation> for u8 {
    fn from(location: ScriptLocation) -> Self {
        match location {
            ScriptLocation::InputLock => 0,
            ScriptLocation::InputType => 1,
            ScriptLocation::OutputType => 2,
            ScriptLocation::DynamicLinking => 3,
            ScriptLocation::Spawn => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScriptAttr {
    pub location: ScriptLocation,
    pub script_hash: [u8; 32],
    pub script: Option<Script>,
}

impl ScriptAttr {
    pub fn validate(&self) -> Result<(), Error> {
        match self.location {
            ScriptLocation::InputLock | ScriptLocation::InputType | ScriptLocation::OutputType => {
                if self.script.is_some() {
                    return Err(Error::InvalidScriptShape);
                }
            }
            ScriptLocation::DynamicLinking | ScriptLocation::Spawn => {
                let script = self.script.as_ref().ok_or(Error::InvalidScriptShape)?;
                let script_hash: [u8; 32] = script.calc_script_hash().unpack();
                if script_hash != self.script_hash {
                    return Err(Error::InvalidScriptHash);
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn unpack_script_attr_opt(
    opt: generated::metadata::ScriptAttrOpt,
) -> Result<Option<ScriptAttr>, Error> {
    opt.to_opt().map(unpack_script_attr).transpose()
}

pub(crate) fn unpack_script_attr(
    attr: generated::metadata::ScriptAttr,
) -> Result<ScriptAttr, Error> {
    let parsed = ScriptAttr {
        location: ScriptLocation::try_from(u8::from(attr.location()))?,
        script_hash: attr.script_hash().into(),
        script: attr
            .script()
            .to_opt()
            .map(|script| Script::from_slice(script.as_slice()))
            .transpose()?,
    };
    parsed.validate()?;
    Ok(parsed)
}

pub(crate) fn pack_script_attr_opt(
    attr: &Option<ScriptAttr>,
) -> Result<generated::metadata::ScriptAttrOpt, Error> {
    Ok(generated::metadata::ScriptAttrOpt::new_builder()
        .set(match attr {
            Some(attr) => Some(pack_script_attr(attr)?),
            None => None,
        })
        .build())
}

pub(crate) fn pack_script_attr(
    attr: &ScriptAttr,
) -> Result<generated::metadata::ScriptAttr, Error> {
    attr.validate()?;
    let script_opt = generated::blockchain::ScriptOpt::new_builder()
        .set(
            attr.script
                .as_ref()
                .map(|script| generated::blockchain::Script::from_slice(script.as_slice()))
                .transpose()?,
        )
        .build();

    Ok(generated::metadata::ScriptAttr::new_builder()
        .location(u8::from(attr.location).into())
        .script_hash(generated::blockchain::Byte32::from_slice(
            &attr.script_hash,
        )?)
        .script(script_opt)
        .build())
}

pub(crate) fn unpack_script_attr_vec(
    raw: generated::metadata::ScriptAttrVec,
) -> Result<Vec<ScriptAttr>, Error> {
    if raw.len() > MAX_EXTENSIONS {
        return Err(Error::ExtensionsTooMany);
    }

    let extensions = raw
        .into_iter()
        .map(unpack_script_attr)
        .collect::<Result<Vec<_>, Error>>()?;
    validate_extensions(&extensions)?;
    Ok(extensions)
}

pub(crate) fn pack_script_attr_vec(
    extensions: &[ScriptAttr],
) -> Result<generated::metadata::ScriptAttrVec, Error> {
    validate_extensions(extensions)?;
    let mut builder = generated::metadata::ScriptAttrVec::new_builder();
    for attr in extensions {
        builder = builder.push(pack_script_attr(attr)?);
    }
    Ok(builder.build())
}

pub(crate) fn validate_extensions(extensions: &[ScriptAttr]) -> Result<(), Error> {
    if extensions.len() > MAX_EXTENSIONS {
        return Err(Error::ExtensionsTooMany);
    }

    let mut prev_key: Option<(u8, [u8; 32])> = None;
    for extension in extensions {
        extension.validate()?;
        let key = (u8::from(extension.location), extension.script_hash);
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
