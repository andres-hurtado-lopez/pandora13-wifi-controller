pub enum ControlMessages{
    A,
    B,
    C,
    D,
    E,
    F,
    Up,
    Down,
    Left,
    Right,
    Start,
}


impl ControlMessages{
    pub fn from_byte(i: u8) -> Option<Self> {
	match i {
	    b'y' => Some(Self::A),
	    b'u' => Some(Self::B),
	    b'i' => Some(Self::C),
	    b'h' => Some(Self::D),
	    b'j' => Some(Self::E),
	    b'k' => Some(Self::F),
	    b'w' => Some(Self::Up),
	    b's' => Some(Self::Down),
	    b'a' => Some(Self::Left),
	    b'd' => Some(Self::Right),
	    b'\n' => Some(Self::Start),
	    _ => None
	}
    }
}
