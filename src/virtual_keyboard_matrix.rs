
pub enum VirtualKeyaboardMatrixItem{
    N1,
    N2,
    N3,
    N4,
    N5,
    N6,
    N7,
    N8,
    N9,
    N0,
    Q,
    W,
    E,
    R,
    T,
    Y,
    U,
    I,
    O,
    P,
    A,
    S,
    D,
    F,
    G,
    H,
    J,
    K,
    L,
    Trash,
    Z,
    X,
    C,
    V,
    Space,
    B,
    N,
    M,
    Dash,
    Backspace,
    EOL,
    Quit
}

pub struct VirtualKeyaboardMatrixBufferIterator<'a>{
    index: usize,
    buf: &'a[u8]
}

impl <'a> Iterator for VirtualKeyaboardMatrixBufferIterator<'a>{
    type Item = VirtualKeyaboardMatrixItem;

    fn next(&mut self) -> Option<Self::Item> {
	self
	    .buf
	    .get(self.index)
	    .and_then(|x| VirtualKeyaboardMatrixItem::byte_to_matrix(*x) )
    }
}

impl VirtualKeyaboardMatrixItem{

    pub fn convert_bytes(i: &[u8]) -> VirtualKeyaboardMatrixBufferIterator {

	VirtualKeyaboardMatrixBufferIterator{
	    index:0,
	    buf: i,
	}
	
    }
    
    pub fn byte_to_matrix(i:u8) -> Option<Self>{
	match i {
	    b'1' => Some(Self::N1),
	    b'2' => Some(Self::N2),
	    b'3' => Some(Self::N3),
	    b'4' => Some(Self::N4),
	    b'5' => Some(Self::N5),
	    b'6' => Some(Self::N6),
	    b'7' => Some(Self::N7),
	    b'8' => Some(Self::N8),
	    b'9' => Some(Self::N9),
	    b'0' => Some(Self::N0),
	    b'q' | b'Q' => Some(Self::Q),
	    b'w' | b'W' => Some(Self::W),
	    b'e' | b'E' => Some(Self::E),
	    b'r' | b'R' => Some(Self::R),
	    b't' | b'T' => Some(Self::T),
	    b'y' | b'Y' => Some(Self::Y),
	    b'u' | b'U' => Some(Self::U),
	    b'i' | b'I' => Some(Self::I),
	    b'o' | b'O' => Some(Self::O),
	    b'p' | b'P' => Some(Self::P),
	    b'a' | b'A' => Some(Self::A),
	    b's' | b'S' => Some(Self::S),
	    b'd' | b'D' => Some(Self::D),
	    b'f' | b'F' => Some(Self::F),
	    b'g' | b'G' => Some(Self::G),
	    b'h' | b'H' => Some(Self::H),
	    b'j' | b'J' => Some(Self::J),
	    b'k' | b'K' => Some(Self::K),
	    b'l' | b'L' => Some(Self::L),
	    b'z' | b'Z' => Some(Self::Z),
	    b'x' | b'X' => Some(Self::X),
	    b'c' | b'C' => Some(Self::C),
	    b'v' | b'V' => Some(Self::V),
	    b' '  => Some(Self::Space),
	    b'b' | b'B' => Some(Self::B),
	    b'n' | b'N' => Some(Self::N),
	    b'm' | b'M' => Some(Self::M),
	    b'-' => Some(Self::Dash),
	    b'\n' => Some(Self::EOL),
	    1 => Some(Self::Trash),
	    2 => Some(Self::Backspace),
	    3 => Some(Self::Quit),
	    _ => None,
	}
    }
    
    fn location(&self) -> Option<(i8, i8)> {
	match self {
	    Self::N1 => Some((0,0)),
	    Self::N2 => Some((0,1)),
	    Self::N3 => Some((0,2)),
	    Self::N4 => Some((0,3)),
	    Self::N5 => Some((0,4)),
	    Self::N6 => Some((0,5)),
	    Self::N7 => Some((0,6)),
	    Self::N8 => Some((0,7)),
	    Self::N9 => Some((0,8)),
	    Self::N0 => Some((0,9)),
	    
	    Self::Q => Some((1,0)),
	    Self::W => Some((1,1)),
	    Self::E => Some((1,2)),
	    Self::R => Some((1,3)),
	    Self::T => Some((1,4)),
	    Self::Y => Some((1,5)),
	    Self::U => Some((1,6)),
	    Self::I => Some((1,7)),
	    Self::O => Some((1,8)),
	    Self::P => Some((1,9)),
	    
	    Self::A => Some((2,0)),
	    Self::S => Some((2,1)),
	    Self::D => Some((2,2)),
	    Self::F => Some((2,3)),
	    Self::G => Some((2,4)),
	    Self::H => Some((2,5)),
	    Self::J => Some((2,6)),
	    Self::K => Some((2,7)),
	    Self::L => Some((2,8)),
	    Self::Trash => Some((2,9)),
	    
	    Self::Z => Some((3,0)),
	    Self::X => Some((3,1)),
	    Self::C => Some((3,2)),
	    Self::V => Some((3,3)),
	    Self::Space => Some((3,4)),
	    Self::B => Some((3,5)),
	    Self::N => Some((3,6)),
	    Self::M => Some((3,7)),
	    Self::Dash => Some((3,8)),
	    Self::Backspace => Some((3,9)),
	    _ => None,
	}
    }

    pub fn compute_move_delta(&self, to: &Self) -> Option<(i8, i8)>{

	let Some((self_x, self_y)) = self.location() else {
	    return None
	};

	let Some((to_x, to_y)) = to.location() else {
	    return None
	};


        Some((to_x - self_x, to_y - self_y ))

    }
}

