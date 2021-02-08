use log::*;
use std::convert::From;

use super::helpers::*;
use crate::text::*;
use nom::combinator;

#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
pub enum Elem {
    Char(char),
    Alt(char),
    Control(char),
    Resize(usize, usize),
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
    Backspace,
    Delete,
    Tab,
}

impl Elem {
    fn is_digit(&self, radix: u32) -> bool {
        match self {
            Self::Char(c) if c.is_digit(radix) => true,
            _ => false,
        }
    }

    fn into_char(&self) -> Option<char> {
        match self {
            Self::Char(c) => Some(*c),
            //Self::Enter => Some('\n'),
            Self::Tab => Some('\t'),
            _ => None,
        }
    }
}

fn range_enter(s: &str) -> Vec<Elem> {
    let mut v = range_string(s);
    v.push(Elem::Enter);
    v
}

fn range_string(s: &str) -> Vec<Elem> {
    s.chars().map(|x| Elem::Char(x)).collect::<Vec<Elem>>()
}

#[derive(Debug)]
pub struct TokenError {}

impl TryInto<Command> for Event {
    type Error = TokenError;
    fn try_into(self) -> Result<Command, TokenError> {
        use crossterm::event::*;
        match self {
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
            Event::Key(KeyEvent {
                code: KeyCode::Char('m'),
                modifiers: KeyModifiers::CONTROL,
            }) => Ok(Command::Test),
            Event::Key(KeyEvent {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::CONTROL,
            }) => Ok(Command::Refresh),
            Event::Key(KeyEvent {
                code: KeyCode::Char('z'),
                modifiers: KeyModifiers::CONTROL,
            }) => Ok(Command::Stop),
            _ => Err(TokenError {}),
        }
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

type Range<'a> = &'a [Elem];

struct R<'a>(Range<'a>);

impl<'a> R<'a> {
    fn take(count: usize) -> impl FnMut(Range) -> IResult<Range, Range> {
        move |i: Range| {
            if i.len() >= count {
                Ok((&i[count..], &i[..count]))
            } else {
                info!("take - incomplete: {:?}", i);
                Err(Err::Incomplete(Needed::Unknown))
            }
        }
    }

    // get a single char
    fn char() -> impl FnMut(Range) -> IResult<Range, char> {
        |i| Self::p_char(i)
    }

    fn p_char(i: Range) -> IResult<Range, char> {
        if i.len() == 0 {
            info!("char - incomplete: {:?}", i);
            return Err(Err::Incomplete(Needed::new(1)));
        }
        match i[0] {
            Elem::Char(ch) => Ok((&i[1..], ch)),
            _ => Err(Err::Error(Error::new(i, ErrorKind::Tag))),
        }
        //let s: String = i.iter().map(|t| t.into_char())
        //.take_while(|t| t.is_some())
        //.map(|t| t.unwrap())
        //.take(1)
        //.collect::<String>();
        //match s.chars().next() {
        //Some(c) => Ok((&i[1..], c)),
        //None => {
        //info!("char - incomplete: {:?}", s);
        //Err(Err::Incomplete(Needed::new(1)))
        //}
        //}
    }

    fn take_string(count: usize) -> impl FnMut(Range) -> IResult<Range, String> {
        move |i: Range| {
            let c = count.clone();
            let s = i
                .iter()
                .map(|t| t.into_char())
                .take_while(|t| t.is_some())
                .map(|t| t.unwrap())
                .take(c)
                .collect::<String>();
            Ok((&i[s.len()..], s))
        }
    }

    pub fn p_string(i: Range<'a>) -> IResult<Range<'a>, String> {
        let p = nom::multi::many0(R::char());
        combinator::map(p, |v| v.iter().collect::<String>())(i)
    }

    fn p_string2(i: Range<'a>) -> IResult<Range<'a>, String> {
        Self::take_string_while(|_x| true)(i)
    }

    fn string() -> impl Fn(Range<'a>) -> IResult<Range<'a>, String> {
        |i| Self::p_string(i)
    }

    fn take_string_while(
        f: impl Fn(&char) -> bool,
    ) -> impl Fn(Range<'a>) -> IResult<Range<'a>, String> {
        move |i| Self::p_string_while(i, |x| f(x))
    }

    fn p_string_while(i: Range, f: impl Fn(&char) -> bool) -> IResult<Range, String> {
        let s = i
            .iter()
            .map(|t| t.into_char())
            .take_while(|t| t.is_some())
            .map(|t| t.unwrap())
            .take_while(f)
            .collect::<String>();
        Ok((&i[s.len()..], s))
    }

    fn tag_string(r: &str) -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Range<'a>> {
        Self::tag_elem(range_string(r))
    }

    fn tag_elem(r: Vec<Elem>) -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Range<'a>> {
        let s = r.clone();
        move |i| {
            let len = s.len();
            let s_incomplete = &s[..std::cmp::min(len, i.len())];
            if i.len() < s.len() && s_incomplete == i {
                info!("tag_elem - incomplete: {:?}", s);
                Err(Err::Incomplete(Needed::new(s.len() - i.len())))
            } else if i.len() >= s.len() && &i[..len] == r {
                Ok((&i[len..], &i[..len]))
            } else {
                Err(Err::Error(Error::new(i, ErrorKind::Tag)))
            }
        }
    }

    fn tag(r: Range<'a>) -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Range<'a>> {
        let s = &(*r);
        move |i| {
            let len = s.len();
            let s_incomplete = &s[..std::cmp::min(len, i.len())];
            if i.len() < s.len() && s_incomplete == i {
                info!("tag - incomplete: {:?}", s);
                Err(Err::Incomplete(Needed::new(s.len() - i.len())))
            } else if i.len() >= s.len() && &i[..len] == r {
                Ok((&i[len..], &i[..len]))
            } else {
                Err(Err::Error(Error::new(i, ErrorKind::Tag)))
            }
        }
    }

    pub fn number<O>() -> impl FnMut(Range<'a>) -> IResult<Range<'a>, O>
    where
        O: std::str::FromStr,
    {
        |i| Self::p_number(i)
    }
    pub fn p_number<O>(i: Range<'a>) -> IResult<Range<'a>, O>
    where
        O: std::str::FromStr,
    {
        let digit = Self::take_string_while(|e| e.is_digit(10));
        map_opt(digit, |s: String| s.parse::<O>().ok())(i)
    }

    fn oneof(choices: Range<'a>) -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Elem> {
        move |i: Range| {
            let ch = &(*choices);
            match i.iter().next().map(|c| (c, ch.iter().find(|e| *e == c))) {
                None => {
                    //info!("oneof - incomplete: {:?}", &choices);
                    Err(Err::Incomplete(Needed::new(1)))
                }
                Some((_, None)) => Err(Err::Error(Error::new(i, ErrorKind::OneOf))),
                Some((_, Some(_))) => Ok((&i[1..], i[0])),
            }
        }
    }

    fn string_until(until: Range<'a>) -> impl FnMut(Range<'a>) -> IResult<Range<'a>, String> {
        move |i| {
            match tuple((R::string(), R::tag(until)))(i) {
                Ok((rest, (s, _x))) => Ok((rest, s)),
                Err(e) => Err(e),
            }
            //Self::p_string_until(i, until)
        }
    }

    //fn p_string_until(i: Range, until: Elem) -> IResult<Range, String> {
    //match tuple((R::string(), R::tag(&[until])))(i) {
    //Ok((rest, (s, _))) => Ok((rest, s)),
    //Err(e) => Err(e)
    //}

    ////let s = i.iter()
    ////.take_while(|t| **t != until)
    ////.filter_map(|t| t.into_char())
    ////.collect::<String>();
    ////Ok((&i[s.len()..], s))
    //}
    //
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ModeState {
    pub mode: Mode,
    pub record: Option<MacroId>,
    pub macros: Macros,
}
impl Default for ModeState {
    fn default() -> Self {
        Self {
            mode: Mode::Normal,
            record: None,
            macros: Macros::default(),
        }
    }
}

impl<'a> ModeState {
    pub fn change_mode(&mut self, m: Mode) -> &mut Self {
        self.mode = m;
        self
    }

    pub fn command(&self, i: Range<'a>) -> IResult<Range<'a>, Vec<Command>> {
        match self.mode {
            Mode::Normal => self.p_normal(i),
            Mode::Insert => Self::p_insert(i),
            Mode::Easy => self.p_normal(i),
            Mode::Cli => Self::p_cli(i),
        }
    }

    pub fn clear(&mut self) {
        self.macros.clear_all();
        self.record = None;
    }

    pub fn macros_add(&mut self, c: Command) {
        if let Some(id) = self.record {
            self.macros.add(&id, &c);
        }
    }

    fn p_common(i: Range<'a>) -> IResult<Range<'a>, Vec<Command>> {
        alt((
            value(Command::Save.into(), R::oneof(&[Elem::Control('s')])),
            value(Command::LineNav(0).into(), R::oneof(&[Elem::Control('a')])),
            value(Command::LineNav(-1).into(), R::oneof(&[Elem::Control('e')])),
            value(
                Command::ScrollPage(-1).into(),
                R::oneof(&[Elem::Control('u')]),
            ),
            value(
                Command::ScrollPage(1).into(),
                R::oneof(&[Elem::Control('d')]),
            ),
            value(Command::Scroll(-1).into(), R::oneof(&[Elem::Control('f')])),
            value(Command::Scroll(1).into(), R::oneof(&[Elem::Control('b')])),
        ))(i)
    }

    fn alias() -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Vec<Elem>> {
        alt((
            value(range_string("dw"), R::tag_elem(range_string("asdf"))),
            value(range_string("dw"), R::tag_elem(range_string("asdf"))),
        ))
    }

    fn p_alias(i: Range<'a>) -> IResult<Range<'a>, Vec<Elem>> {
        Self::alias()(i)
    }

    fn p_normal(&self, i: Range<'a>) -> IResult<Range<'a>, Vec<Command>> {
        alt((
            value(Command::Quit.into(), R::oneof(&[Elem::Control('q')])),
            combinator::map_opt(Self::alias(), |v: Vec<Elem>| {
                match self.p_unmapped_normal(v.as_slice()) {
                    Ok((_, x)) => Some(x),
                    Err(_) => None,
                }
            }),
            |x| self.p_unmapped_normal(x),
        ))(i)
    }

    fn p_unmapped_normal(&self, i: Range<'a>) -> IResult<Range<'a>, Vec<Command>> {
        use Command as C;
        alt((
            map(
                tuple((R::number(), R::oneof(&[Elem::Enter, Elem::Char('G')]))),
                |x| C::Line(x.0).into(),
            ),
            map_opt(
                tuple((T::range(), R::oneof(&[Elem::Enter]))),
                |(x, _)| match x {
                    T::Range(_, b) => Some(C::Line(b as i64).into()),
                    _ => None,
                },
            ),
            T::cli(),
            value(C::Mode(Mode::Insert).into(), R::tag_string("i")),
            value(C::Line(0).into(), R::tag_string("G")),
            value(C::Line(1).into(), R::tag_string("gg")),
            value(C::Join.into(), R::tag_string("J")), // Join
            value(C::ChangeRepeat.into(), R::tag(&[Elem::Char('.')])), // Change Repeat
            value(
                vec![
                    C::Motion(1, Motion::NextLine),
                    C::Mode(Mode::Insert),
                    C::Insert('\n'),
                    C::Motion(1, Motion::Left),
                ],
                R::tag_string("o"),
            ),
            value(
                vec![
                    C::Motion(1, Motion::SOL),
                    C::Mode(Mode::Insert),
                    C::Insert('\n'),
                    C::Motion(1, Motion::Left),
                ],
                R::tag_string("O"),
            ),
            value(C::BufferNext.into(), R::tag_string("]")),
            value(C::BufferPrev.into(), R::tag_string("[")),
            |i| Self::p_common(i),
            T::motion(),
            //combinator::peek(T::search_inc()),
            //T::search_inc(),
            //T::search(),
            T::operator_motion(),
            T::register_motion(),
            //|i| T::p_macros(i, self.record),
        ))(i)
    }

    fn p_insert(i: Range<'a>) -> IResult<Range<'a>, Vec<Command>> {
        alt((
            value(
                vec![Command::Mode(Mode::Normal), Command::ChangeEnd],
                R::oneof(&[Elem::Control('c'), Elem::Esc]),
            ),
            value(Command::Save.into(), R::oneof(&[Elem::Control('s')])),
            value(Command::Quit.into(), R::oneof(&[Elem::Control('q')])),
            map(complete(R::char()), |x| Command::Insert(x).into()),
            value(Command::Motion(1, Motion::Up).into(), R::tag(&[Elem::Up])),
            value(
                Command::Motion(1, Motion::Down).into(),
                R::tag(&[Elem::Down]),
            ),
            value(
                Command::Motion(1, Motion::Left).into(),
                R::tag(&[Elem::Left]),
            ),
            value(
                Command::Motion(1, Motion::Right).into(),
                R::tag(&[Elem::Right]),
            ),
            value(Command::RemoveChar(-1).into(), R::tag(&[Elem::Backspace])),
            value(Command::RemoveChar(1).into(), R::tag(&[Elem::Delete])),
            value(Command::Insert('\n').into(), R::tag(&[Elem::Enter])),
            value(Command::Insert('\t').into(), R::tag(&[Elem::Tab])),
            //map(R::char(), |x| Command::Insert(x).into()),
        ))(i)
    }

    fn p_cli(i: Range<'a>) -> IResult<Range<'a>, Vec<Command>> {
        use Command as C;
        use Elem as E;
        alt((
            value(Command::Quit.into(), R::oneof(&[E::Control('q')])),
            value(
                vec![C::Mode(Mode::Normal), C::CliCancel],
                R::oneof(&[E::Esc, E::Control('c')]),
            ),
            value(vec![C::Mode(Mode::Normal), C::CliExec], R::tag(&[E::Enter])),
            value(
                C::CliEdit(C::RemoveChar(-1).into()).into(),
                R::tag(&[E::Backspace]),
            ),
            value(
                C::CliEdit(C::RemoveChar(1).into()).into(),
                R::tag(&[E::Delete]),
            ),
            map(R::char(), |ch| C::CliEdit(C::Insert(ch).into()).into()),
        ))(i)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Motion {
    OnCursor,
    AfterCursor,
    Left,
    Right,
    Up,
    Down,
    // Line
    EOL,
    SOL,
    NextLine,
    SOLT, // start of line text
    Line,
    AbsLine,

    ForwardWord1,
    ForwardWord2,
    ForwardWordEnd1,
    ForwardWordEnd2,
    BackWord1,
    BackWord2,
    NextWord,
    EOW,
    PrevWord,
    SOW,
    NextSearch,
    PrevSearch,
    Til1(char),
    Til2(char),
    // start and end of buffer
    SOB,
    EOB,
}
impl Motion {
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

    fn p_motion(i: Range) -> IResult<Range, Self> {
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

    fn motion() -> impl FnMut(Range) -> IResult<Range, Self> {
        |i| Self::p_motion(i)
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
enum T {
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
    fn range() -> impl FnMut(Range<'a>) -> IResult<Range<'a>, T> {
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

    fn cli() -> impl FnMut(Range) -> IResult<Range, Vec<Command>> {
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

    fn motion() -> impl FnMut(Range) -> IResult<Range, Vec<Command>> {
        |i: Range| match tuple((opt(R::number()), Motion::motion()))(i) {
            Ok((rest, (d1, m))) => {
                let reps: usize = d1.unwrap_or(1);
                Ok((rest, Command::Motion(reps, m).into()))
            }
            Err(e) => Err(e),
        }
    }

    fn register_motion() -> impl FnMut(Range) -> IResult<Range, Vec<Command>> {
        |i| Self::p_register_motion(i)
    }

    fn p_register_motion(i: Range<'a>) -> IResult<Range<'a>, Vec<Command>> {
        use Command as C;
        
        let char_motion = tuple((R::char(), Motion::motion()));
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

    fn operator_motion() -> impl FnMut(Range) -> IResult<Range, Vec<Command>> {
        |i| Self::p_operator_motion(i)
    }

    fn register() -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Register> {
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
            R::oneof(&[Char('d'), Char('c')]),
            Motion::motion(),
        ));
        let dd = tuple((Self::number_or(1), R::tag_string("dd")));
        let x = tuple((Self::number_or(1), R::tag_string("x")));
        let paste = tuple((
            Self::number_or(1),
            Self::register_or(Register('x')),
            R::oneof(&[Char('p'), Char('P'), Alt('v')]),
        ));
        alt((
            combinator::map_opt(paste, |(reps, reg, op)| match op {
                Alt('p') => Some(vec![
                    C::ChangeStart,
                    C::Paste(reps, reg, M::OnCursor),
                    C::ChangeEnd,
                ]),
                Char('P') => Some(vec![
                    C::ChangeStart,
                    C::Paste(reps, reg, M::SOL),
                    C::ChangeEnd,
                ]),
                Char('p') => Some(vec![
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
                Char('d') => Some(vec![C::ChangeStart, C::Delete(reps, m), C::ChangeEnd]),
                Char('c') => Some(vec![
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
