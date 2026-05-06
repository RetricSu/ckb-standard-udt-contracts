#[cfg(all(not(feature = "std"), feature = "no-std"))]
use ckb_std::ckb_types::{packed::Script, prelude::*};
#[cfg(feature = "std")]
use ckb_types::{packed::Script, prelude::*};
use molecule::prelude::{Builder, Entity};

use crate::{error::Error, generated};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityType {
    InputLock,
    InputType,
    OutputType,
    DynamicLinking,
    Spawn,
}

impl TryFrom<u8> for AuthorityType {
    type Error = Error;

    fn try_from(authority_type: u8) -> Result<Self, Self::Error> {
        match authority_type {
            0 => Ok(Self::InputLock),
            1 => Ok(Self::InputType),
            2 => Ok(Self::OutputType),
            3 => Ok(Self::DynamicLinking),
            4 => Ok(Self::Spawn),
            _ => Err(Error::InvalidScriptLocation),
        }
    }
}

impl From<AuthorityType> for u8 {
    fn from(authority_type: AuthorityType) -> Self {
        match authority_type {
            AuthorityType::InputLock => 0,
            AuthorityType::InputType => 1,
            AuthorityType::OutputType => 2,
            AuthorityType::DynamicLinking => 3,
            AuthorityType::Spawn => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Authority {
    pub authority_type: AuthorityType,
    pub script_hash: [u8; 32],
    pub script: Option<Script>,
}

impl Authority {
    pub fn validate(&self) -> Result<(), Error> {
        match self.authority_type {
            AuthorityType::InputLock | AuthorityType::InputType | AuthorityType::OutputType => {
                if self.script.is_some() {
                    return Err(Error::InvalidScriptShape);
                }
            }
            AuthorityType::DynamicLinking | AuthorityType::Spawn => {
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

pub(crate) fn unpack_authority_opt(
    opt: generated::metadata::AuthorityOpt,
) -> Result<Option<Authority>, Error> {
    opt.to_opt().map(unpack_authority).transpose()
}

pub(crate) fn unpack_authority(
    authority: generated::metadata::Authority,
) -> Result<Authority, Error> {
    let parsed = Authority {
        authority_type: AuthorityType::try_from(u8::from(authority.authority_type()))?,
        script_hash: authority.script_hash().into(),
        script: authority
            .script()
            .to_opt()
            .map(|script| Script::from_slice(script.as_slice()))
            .transpose()?,
    };
    parsed.validate()?;
    Ok(parsed)
}

pub(crate) fn pack_authority_opt(
    authority: &Option<Authority>,
) -> Result<generated::metadata::AuthorityOpt, Error> {
    Ok(generated::metadata::AuthorityOpt::new_builder()
        .set(match authority {
            Some(authority) => Some(pack_authority(authority)?),
            None => None,
        })
        .build())
}

pub(crate) fn pack_authority(
    authority: &Authority,
) -> Result<generated::metadata::Authority, Error> {
    authority.validate()?;
    let script_opt = generated::blockchain::ScriptOpt::new_builder()
        .set(
            authority
                .script
                .as_ref()
                .map(|script| generated::blockchain::Script::from_slice(script.as_slice()))
                .transpose()?,
        )
        .build();

    Ok(generated::metadata::Authority::new_builder()
        .authority_type(u8::from(authority.authority_type).into())
        .script_hash(generated::blockchain::Byte32::from_slice(
            &authority.script_hash,
        )?)
        .script(script_opt)
        .build())
}
