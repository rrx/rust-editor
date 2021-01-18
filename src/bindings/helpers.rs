pub use nom::number::streaming::double;
pub use nom::character::complete::{char, digit1, digit0};
pub use nom::{IResult, bytes, Needed, Err, Finish, error::{make_error, Error,ErrorKind}};
pub use nom::sequence::{pair, tuple};
pub use nom::branch::alt;
pub use nom::combinator::{complete, value, map, map_opt, map_res, opt};
pub use nom::bytes::complete::tag;
pub use crossterm::event::{KeyCode, KeyModifiers, KeyEvent, Event};
pub use std::convert::TryInto;
use std::str::FromStr;

pub fn number<O>(i: &str) -> IResult<&str, O>
where O: FromStr
{
    println!("X:{:?}\r", i);
    map_opt(digit1, |s: &str| s.parse::<O>().ok())(i)
}

pub fn decimal(i: &str) -> IResult<&str, usize> {
    println!("X:{:?}\r", i);
    map_opt(digit1, |s: &str| s.parse::<usize>().ok())(i)
}

pub trait NodeParser<I, O> {
    fn parse(i: I) -> IResult<I, O>;
}


