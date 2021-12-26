use log::*;
use std::convert::From;
use super::helpers::*;
use editor_core::{MacroId, Command, Register, Motion, Mode, Macros};
use nom::combinator;
use crossterm::event::Event;
use std::convert::TryFrom;
use super::range::{Range, Elem, R, range_enter};
use crate::*;

#[derive(Debug)]
pub struct TokenError {}


pub fn event_to_command(event: Event) -> Result<Command, TokenError> {
    use crossterm::event::*;
    match event {
        Event::Resize(x, y) => Ok(Command::Resize(x, y)),
        Event::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: _,
        }) => match kind {
            MouseEventKind::ScrollUp => Ok(Command::Scroll(1)),
            MouseEventKind::ScrollDown => Ok(Command::Scroll(-1)),
            MouseEventKind::Moved => Ok(Command::Mouse(column, row)),
            _ => Err(TokenError {}),
        },
        _ => Err(TokenError {}),
    }
}

impl TryInto<Elem> for Event {
    type Error = TokenError;
    fn try_into(self) -> Result<Elem, TokenError> {
        match self {
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::CONTROL,
            }) => Ok(Elem::Control(c)),
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::ALT,
            }) => Ok(Elem::Alt(c)),
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE,
            }) => Ok(Elem::Char(c)),
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::SHIFT,
            }) => {
                if c.is_ascii() {
                    Ok(Elem::Char(c.to_ascii_uppercase()))
                } else {
                    Ok(Elem::Char(c))
                }
            }
            Event::Key(KeyEvent { code, modifiers: _ }) => match code {
                KeyCode::Enter => Ok(Elem::Enter),
                KeyCode::Esc => Ok(Elem::Esc),
                KeyCode::Backspace => Ok(Elem::Backspace),
                KeyCode::Delete => Ok(Elem::Delete),
                KeyCode::Tab => Ok(Elem::Tab),
                KeyCode::Up => Ok(Elem::Up),
                KeyCode::Down => Ok(Elem::Down),
                KeyCode::Left => Ok(Elem::Left),
                KeyCode::Right => Ok(Elem::Right),
                _ => Err(TokenError {}),
            },
            _ => Err(TokenError {}),
        }
    }
}
impl TryInto<Elem> for &Event {
    type Error = TokenError;
    fn try_into(self) -> Result<Elem, TokenError> {
        let event: Event = *self;
        event.try_into()
    }
}


struct MotionParse(Motion);

impl MotionParse {
    fn _next(i: Range) -> Option<Motion> {
        match i.iter().next() {
            Some(Elem::Char(c)) => match c {
                'h' => Some(Motion::Left),
                'j' => Some(Motion::Down),
                'k' => Some(Motion::Up),
                'l' => Some(Motion::Right),
                'w' => Some(Motion::ForwardWord1),
                'W' => Some(Motion::ForwardWord2),
                'b' => Some(Motion::BackWord1),
                'B' => Some(Motion::BackWord2),
                'e' => Some(Motion::ForwardWordEnd1),
                'E' => Some(Motion::ForwardWordEnd2),
                'n' => Some(Motion::NextSearch),
                'N' => Some(Motion::PrevSearch),
                '$' => Some(Motion::EOL),
                '^' => Some(Motion::SOLT),
                '0' => Some(Motion::SOL),
                _ => None,
            },
            _ => None,
        }
    }

    fn p_motion(i: Range) -> IResult<Range, Motion> {
        alt((
            map(tuple((R::tag_string("t"), R::char())), |(_, ch)| {
                Motion::Til1(ch)
            }),
            map(tuple((R::tag_string("T"), R::char())), |(_, ch)| {
                Motion::Til2(ch)
            }),
            map_opt(R::take(1), Self::_next),
        ))(i)
    }

    fn motion() -> impl FnMut(Range) -> IResult<Range, Motion> {
        |i| Self::p_motion(i)
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum T {
    Number(usize),
    Command(Command),
    Range(usize, usize),
}
impl From<Command> for T {
    fn from(item: Command) -> Self {
        T::Command(item)
    }
}

impl<'a> T {
    pub fn range() -> impl FnMut(Range<'a>) -> IResult<Range<'a>, T> {
        |i| Self::p_range(i)
    }

    fn p_range(r: Range<'a>) -> IResult<Range<'a>, T> {
        // [number],[number]
        match tuple((R::number(), R::tag(&[Elem::Char(',')]), R::number()))(r) {
            Ok((rest, v)) => Ok((rest, T::Range(v.0, v.2))),
            Err(e) => Err(e),
        }
    }

    pub fn _number() -> impl FnMut(Range<'a>) -> IResult<Range<'a>, T> {
        |i| map(R::number(), |n: usize| T::Number(n))(i)
    }

    fn string() -> impl FnMut(Range<'a>) -> IResult<Range<'a>, String> {
        let p = nom::multi::many0(R::char());
        combinator::map(p, |v| v.iter().collect::<String>())
    }

    fn string_inc() -> impl FnMut(Range<'a>) -> IResult<Range<'a>, String> {
        |i| {
            let r = Self::string()(i);
            info!("R: {:?}", (&i, &r));
            match r {
                Ok((rest, s)) => Ok((rest, s)),
                Err(Err::Incomplete(_)) => {
                    //let s = R::p_string(i);
                    info!("S: {:?}", i);
                    Ok((i, "".to_string()))
                }
                Err(e) => Err(e),
            }
        }
    }

    //fn search_inc() -> impl FnMut(Range) -> IResult<Range, Vec<Command>> {
    //use Elem::*;

    //|i: Range| {
    //// Slash + NotEnter + Enter
    //match tuple((
    //R::tag(&[Char('/')]),
    ////combinator::peek(Self::string()),
    ////Self::string_inc(),
    //combinator::peek(R::char()),
    //))(i) {
    //Ok((rest, (_, s))) => {
    //Ok((rest, Command::SearchInc(s.to_string()).into()))
    //}
    ////Err(Err::Incomplete(_)) => {
    ////Ok((i, Command::Quit.into()))
    ////}
    //Err(e) => Err(e)
    //}
    //}
    //}

    //fn search() -> impl FnMut(Range) -> IResult<Range, Vec<Command>> {
    //use Elem::*;
    //|i: Range| {
    //// Slash + NotEnter + Enter
    //match tuple((
    //R::tag(&[Char('/')]),
    //R::string(),
    //R::tag(&[Enter])
    //))(i) {
    //Ok((rest, (_, s, _))) => {
    //Ok((rest, Command::Search(s).into()))
    //}
    //Err(e) => Err(e)
    //}
    //}
    //}

    pub fn cli() -> impl FnMut(Range) -> IResult<Range, Vec<Command>> {
        use Command as C;
        use Mode as M;
        |i| {
            alt((
                value(
                    vec![C::Mode(M::Cli), C::CliEdit(C::Insert('/').into())],
                    R::tag_string("/"),
                ),
                value(
                    vec![C::Mode(M::Cli), C::CliEdit(C::Insert('?').into())],
                    R::tag_string("?"),
                ),
                value(
                    vec![C::Mode(M::Cli), C::CliEdit(C::Insert(':').into())],
                    R::tag_string(":"),
                ),
            ))(i)
        }
    }

    pub fn motion() -> impl FnMut(Range) -> IResult<Range, Vec<Command>> {
        |i: Range| match tuple((opt(R::number()), MotionParse::motion()))(i) {
            Ok((rest, (d1, m))) => {
                let reps: usize = d1.unwrap_or(1);
                Ok((rest, Command::Motion(reps, m).into()))
            }
            Err(e) => Err(e),
        }
    }

    pub fn register_motion() -> impl FnMut(Range) -> IResult<Range, Vec<Command>> {
        |i| Self::p_register_motion(i)
    }

    pub fn p_register_motion(i: Range<'a>) -> IResult<Range<'a>, Vec<Command>> {
        use Command as C;

        let char_motion = tuple((R::char(), MotionParse::motion()));
        let x = Register('x');
        alt((
            combinator::map_opt(
                tuple((Self::register_or(x), char_motion)),
                |(reg, (op, m))| match op {
                    'y' => Some(C::Yank(reg, m).into()),
                    'Y' => Some(C::Yank(reg, m).into()),
                    _ => None,
                },
            ),
            combinator::map(
                tuple((Self::register_or(x), R::tag_string("yy"))),
                |(reg, _)| C::Yank(reg, Motion::Line).into(),
            ),
        ))(i)
    }

    pub fn operator_motion() -> impl FnMut(Range) -> IResult<Range, Vec<Command>> {
        |i| Self::p_operator_motion(i)
    }

    pub fn register() -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Register> {
        combinator::map(tuple((R::tag_string("\""), R::char())), |(_, reg)| {
            Register(reg)
        })
    }

    fn register_or(r: Register) -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Register> {
        move |i| combinator::map(opt(Self::register()), |o| o.unwrap_or(r))(i)
    }

    fn number_or(n: usize) -> impl FnMut(Range<'a>) -> IResult<Range<'a>, usize> {
        move |i| Self::p_number_or(i, n)
    }

    fn p_number_or(i: Range<'a>, n: usize) -> IResult<Range<'a>, usize> {
        combinator::map(opt(R::number()), |o| o.unwrap_or(n))(i)
    }

    fn p_operator_motion(i: Range<'a>) -> IResult<Range<'a>, Vec<Command>> {
        use Command as C;
        use Elem::*;
        use Motion as M;
        let d_motion = tuple((
            Self::number_or(1),
            R::oneof(&[Elem::Char('d'), Elem::Char('c')]),
            MotionParse::motion(),
        ));
        let dd = tuple((Self::number_or(1), R::tag_string("dd")));
        let x = tuple((Self::number_or(1), R::tag_string("x")));
        let paste = tuple((
            Self::number_or(1),
            Self::register_or(Register('x')),
            R::oneof(&[Elem::Char('p'), Elem::Char('P'), Elem::Alt('v')]),
        ));
        alt((
            combinator::map_opt(paste, |(reps, reg, op)| match op {
                Elem::Alt('p') => Some(vec![
                    C::ChangeStart,
                    C::Paste(reps, reg, M::OnCursor),
                    C::ChangeEnd,
                ]),
                Elem::Char('P') => Some(vec![
                    C::ChangeStart,
                    C::Paste(reps, reg, M::SOL),
                    C::ChangeEnd,
                ]),
                Elem::Char('p') => Some(vec![
                    C::ChangeStart,
                    C::Paste(reps, reg, M::NextLine),
                    C::ChangeEnd,
                ]),
                _ => None,
            }),
            combinator::map(x, |(reps, _)| {
                vec![C::ChangeStart, C::Delete(reps, Motion::Right), C::ChangeEnd]
            }),
            combinator::map(dd, |(reps, _)| {
                vec![C::ChangeStart, C::Delete(reps, Motion::Line), C::ChangeEnd]
            }),
            combinator::map_opt(d_motion, |(reps, op, m)| match op {
                Elem::Char('d') => Some(vec![C::ChangeStart, C::Delete(reps, m), C::ChangeEnd]),
                Elem::Char('c') => Some(vec![
                    C::ChangeStart,
                    C::Delete(reps, m),
                    C::Mode(Mode::Insert),
                ]),
                _ => None,
            }),
        ))(i)
    }

    fn p_macros(i: Range<'a>, record: Option<MacroId>) -> IResult<Range<'a>, Vec<Command>> {
        info!("p_macros:{:?}", (i, record));
        use combinator::*;
        //map_opt(alt((
        //cond(record.is_none(), value(Command::MacroEnd.into(), R::tag(&[Elem::Char('q')]))),
        //cond(record.is_some(), map(tuple((R::tag(&[Elem::Char('q')]), R::char())), |(_,ch)| {
        //Command::MacroStart(MacroId(ch)).into()
        //}))
        //)), |v| v)(i)

        match record {
            Some(_) => {
                value(Command::MacroEnd.into(), R::tag(&[Elem::Char('q')]))(i)
                //Self::p_macro_exit(r)
            }
            None => {
                map(tuple((R::tag(&[Elem::Char('q')]), R::char())), |(_, ch)| {
                    Command::MacroStart(MacroId(ch)).into()
                })(i)
                //Self::p_macro_enter(r)
            }
        }
    }

    fn p_macro_enter(i: Range<'a>) -> IResult<Range<'a>, Vec<Command>> {
        combinator::map(tuple((R::tag(&[Elem::Char('q')]), R::char())), |(_, ch)| {
            Command::MacroStart(MacroId(ch)).into()
        })(i)
    }

    fn p_macro_exit(i: Range<'a>) -> IResult<Range<'a>, Vec<Command>> {
        value(Command::MacroEnd.into(), R::tag(&[Elem::Char('q')]))(i)
    }
}

#[derive(Debug)]
pub enum ParseError {
    Incomplete,
    Invalid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_7_c() {
        let r = range_enter("1234a");
        let (rest, v) = R::number::<u64>()(r.as_slice()).unwrap();
        assert_eq!(rest, &[Elem::Char('a'), Elem::Enter]);
        assert_eq!(v, 1234);
    }

    #[test]
    fn test_7_c2() {
        let r = range_enter("a");
        let result = R::number::<u64>()(r.as_slice());
        assert_eq!(result.is_err(), true);
    }

    #[test]
    fn test_7_d() {
        let r = range_enter("1234");
        let (rest, v) = R::string()(r.as_slice()).unwrap();
        println!("R: {:?}", (&rest, &v));
        assert_eq!(rest, &[Elem::Enter]);
        assert_eq!(v, "1234");
    }

    #[test]
    fn test_7_1() {
        let i = range_enter("1234,4321");
        let (_, v) = T::range()(i.as_slice()).unwrap();
        assert_eq!(v, T::Range(1234, 4321));
    }

    #[test]
    fn test_7_3() {
        let i = range_enter("100j");
        let state = ModeState::default();
        let (_, v) = state.command(i.as_slice()).unwrap();
        assert_eq!(v, vec![Command::Motion(100, Motion::Down)]);
    }

    #[test]
    fn test_7_4() {
        let i = range_enter("1234");
        let state = ModeState::default();
        let (_, v) = state.command(i.as_slice()).unwrap();
        assert_eq!(v, vec![Command::Line(1234)]);
    }
}
