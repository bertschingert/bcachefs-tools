mod bkey_types;

use anyhow::Result;

pub fn list_bkeys() -> Result<()> {
    print!("{}", bkey_types::get_bkey_type_info()?);

    Ok(())
}
