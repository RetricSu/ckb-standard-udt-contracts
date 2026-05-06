use crate::error::Error;

pub fn run() -> Result<(), Error> {
    crate::meta_cell::validate_type_args()?;
    let group = crate::meta_cell::load_meta_group()?;

    match (group.input.as_ref(), group.output.as_ref()) {
        (None, Some(output)) => {
            crate::meta_cell::validate_create_type_id()?;
            crate::meta_cell::validate_create(output, &group.meta_type_hash)
        }
        (Some(input), Some(output)) => crate::update::validate_update(input, output),
        _ => Err(Error::InvalidArgs),
    }
}
