use nom::branch::alt;
use nom::bytes::complete::{tag, take_while};
use nom::character::complete::{alpha1, char, space1, u32, u64};
use nom::combinator::{all_consuming, value};
use nom::sequence::tuple;
use nom::IResult;

use bch_bindgen::c::bpos;

use crate::commands::debug::{DebugCommand, DumpCommand, UpdateCommand};

fn parse_bpos(input: &str) -> IResult<&str, bpos> {
    let (input, (inode, _, offset, _, snapshot)) = tuple((
        u64,
        char(':'),
        u64,
        char(':'),
        alt((u32, value(u32::MAX, tag("U32_MAX")))),
    ))(input)?;

    Ok((
        input,
        bpos {
            inode,
            offset,
            snapshot,
        },
    ))
}

fn parse_dump_cmd(input: &str) -> IResult<&str, DebugCommand> {
    let (input, (_, btree, _, bpos)) =
        all_consuming(tuple((space1, alpha1, space1, parse_bpos)))(input)?;

    Ok((
        input,
        DebugCommand::Dump(DumpCommand {
            btree: btree.to_string(),
            bpos,
        }),
    ))
}

fn bkey_name(input: &str) -> IResult<&str, &str> {
    take_while(|c: char| c.is_alphanumeric() || c == '_')(input)
}

fn field_name(input: &str) -> IResult<&str, &str> {
    take_while(|c: char| c.is_alphanumeric() || c == '_' || c == '.')(input)
}

fn parse_update_cmd(input: &str) -> IResult<&str, DebugCommand> {
    let (input, (_, btree, _, bpos, _, bkey, _, field, _, value)) = all_consuming(tuple((
        space1,
        alpha1,
        space1,
        parse_bpos,
        space1,
        bkey_name,
        char('.'),
        field_name,
        char('='),
        u64,
    )))(input)?;

    Ok((
        input,
        DebugCommand::Update(UpdateCommand {
            btree: btree.to_string(),
            bpos,
            bkey: bkey.to_string(),
            field: field.to_string(),
            value,
        }),
    ))
}

fn parse_command_inner(input: &str) -> IResult<&str, DebugCommand> {
    let (input, cmd) = alt((tag("dump"), tag("update")))(input)?;

    match cmd {
        "dump" => parse_dump_cmd(input),
        "update" => parse_update_cmd(input),
        _ => unreachable!(),
    }
}

/// Given an input string, tries to parse it into a valid
/// command to the debug tool.
pub fn parse_command(input: &str) -> anyhow::Result<DebugCommand> {
    match parse_command_inner(input) {
        Ok((_, c)) => Ok(c),
        Err(e) => Err(anyhow::anyhow!("{e}")),
    }
}
