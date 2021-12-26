use super::*;
use nom::combinator::*;
use nom::IResult;
use editor_core::Command;

#[derive(Debug)]
pub enum CommandError {
    Error,
}

fn split(input: &str) -> IResult<&str, Vec<&str>> {
    let parts = input.split_whitespace().collect::<Vec<&str>>();
    Ok(("", parts))
}

fn parse_set(i: Vec<&str>) -> IResult<Vec<&str>, Vec<Command>, CommandError> {
    if i.len() < 1 {
        Err(nom::Err::Error(CommandError::Error))
    } else if i.len() == 1 {
        match i.get(0).unwrap() {
            &"q" | &"quit" => Ok((vec![], Command::Quit.into())),
            &"w" | &"write" => Ok((vec![], Command::Save.into())),
            &"wq" => Ok((vec![], vec![Command::Save, Command::Quit])),
            _ => Err(nom::Err::Error(CommandError::Error)),
        }
    } else if i.len() == 2 {
        let (a, b) = (i.get(0).unwrap(), i.get(1).unwrap());
        match a {
            &"set" => Ok((vec![], Command::VarGet(b.to_string()).into())),
            &"e" | &"edit" => Ok((vec![], Command::Open(b.to_string()).into())),
            &"w" | &"write" => Ok((vec![], Command::SaveAs(b.to_string()).into())),
            _ => Err(nom::Err::Error(CommandError::Error)),
        }
    } else {
        let (v, rest) = i.split_at(3);
        let (a, b, c) = (v.get(0).unwrap(), v.get(1).unwrap(), v.get(2).unwrap());
        if a == &"set" {
            Ok((
                rest.to_vec(),
                Command::VarSet(b.to_string(), c.to_string()).into(),
            ))
        } else {
            Err(nom::Err::Error(CommandError::Error))
        }
    }
}

pub fn command_parse(input: &str) -> Result<Vec<Command>, CommandError> {
    match map_res(split, |s| parse_set(s))(input) {
        Ok((_, (_, commands))) => Ok(commands),
        Err(_err) => Err(CommandError::Error),
    }
}
