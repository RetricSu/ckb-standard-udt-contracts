use alloc::vec::Vec;

use molecule::prelude::{Builder, Entity};

use crate::{error::Error, generated};

use super::config::MAX_ACCESSLIST_ENTRIES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessListRange {
    pub start: [u8; 32],
    pub end: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessListShard {
    pub range: AccessListRange,
    pub entries: Vec<[u8; 32]>,
}

impl AccessListShard {
    pub fn from_slice(data: &[u8]) -> Result<Self, Error> {
        Self::try_from(data)
    }

    pub fn to_molecule(&self) -> Result<generated::metadata::AccessListShard, Error> {
        if self.entries.len() > MAX_ACCESSLIST_ENTRIES {
            return Err(Error::AccessListTooLarge);
        }

        let mut entries = generated::blockchain::Byte32Vec::new_builder();
        for entry in &self.entries {
            entries = entries.push(generated::blockchain::Byte32::from_slice(entry)?);
        }

        Ok(generated::metadata::AccessListShard::new_builder()
            .range(
                generated::metadata::AccessListRange::new_builder()
                    .start(generated::blockchain::Byte32::from_slice(
                        &self.range.start,
                    )?)
                    .end(generated::blockchain::Byte32::from_slice(&self.range.end)?)
                    .build(),
            )
            .entries(entries.build())
            .build())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(self.to_molecule()?.as_slice().to_vec())
    }
}

impl TryFrom<&[u8]> for AccessListShard {
    type Error = Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let raw = generated::metadata::AccessListShard::from_slice(data)?;
        if raw.entries().len() > MAX_ACCESSLIST_ENTRIES {
            return Err(Error::AccessListTooLarge);
        }

        Ok(Self {
            range: AccessListRange {
                start: raw.range().start().into(),
                end: raw.range().end().into(),
            },
            entries: raw.entries().into_iter().map(Into::into).collect(),
        })
    }
}
