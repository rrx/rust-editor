use editor_core::{MacroId, Command, Register, Motion, Mode, Macros};
use nom::combinator::{value, map, complete, map_opt};
use nom::branch::alt;
use nom::sequence::tuple;
use nom::combinator;
use crate::range::{Range, R, Elem, range_string};
use crate::parser::{T};
use nom::IResult;

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
            value(Command::Stop.into(), R::tag(&[Elem::Control('z')])),
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
                    C::Insert("\n".to_string()),
                    C::Motion(1, Motion::Left),
                ],
                R::tag_string("o"),
            ),
            value(
                vec![
                    C::Motion(1, Motion::SOL),
                    C::Mode(Mode::Insert),
                    C::Insert("\n".to_string()),
                    C::Motion(1, Motion::Left),
                ],
                R::tag_string("O"),
            ),
            value(C::BufferNext.into(), R::tag_string("]")),
            value(C::BufferPrev.into(), R::tag_string("[")),
            value(C::Undo.into(), R::tag_string("u")),
            value(C::Redo.into(), R::tag(&[Elem::Control('r')])),
            value(vec![C::Reset, C::Refresh], R::tag_string("RR")),
            value(vec![C::Test], R::tag_string("TT")),
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
            map(complete(R::char()), |x| Command::Insert(x.to_string()).into()),
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
            value(Command::Insert("\n".to_string()).into(), R::tag(&[Elem::Enter])),
            value(Command::Insert("\t".to_string()).into(), R::tag(&[Elem::Tab])),
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
            map(R::char(), |ch| C::CliEdit(C::Insert(ch.to_string()).into()).into()),
        ))(i)
    }
}
