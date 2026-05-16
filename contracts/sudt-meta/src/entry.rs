use crate::error::Error;

pub fn main() -> Result<(), Error> {
    crate::state::validate_type_args()?;
    let group = crate::state::load_meta_group()?;

    match (group.input.as_ref(), group.output.as_ref()) {
        (None, Some(output)) => {
            crate::state::validate_create_type_id()?;
            crate::state::validate_create(output, &group.meta_type_hash)
        }
        (Some(input), Some(output)) => {
            crate::update::validate_update(input, output, &group.meta_type_hash)
        }
        (Some(input), None) => crate::update::validate_destroy(input),
        _ => Err(Error::InvalidArgs),
    }
}
