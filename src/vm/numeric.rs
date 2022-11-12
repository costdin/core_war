use std::ops::{Add, AddAssign, Div, Mul, Rem, Sub, SubAssign};

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct Numeric<const CORE_SIZE: usize> {
    pub value: usize,
}

impl<const CORE_SIZE: usize> From<usize> for Numeric<CORE_SIZE> {
    fn from(item: usize) -> Numeric<CORE_SIZE> {
        Numeric::new(item)
    }
}

impl<const CORE_SIZE: usize> Into<usize> for Numeric<CORE_SIZE> {
    fn into(self) -> usize {
        self.value
    }
}

impl<const CORE_SIZE: usize> Numeric<CORE_SIZE> {
    pub fn new(n: usize) -> Numeric<CORE_SIZE> {
        Numeric::<CORE_SIZE> {
            value: n % CORE_SIZE,
        }
    }
}

impl<const CORE_SIZE: usize> Add<Numeric<CORE_SIZE>> for Numeric<CORE_SIZE> {
    type Output = Numeric<CORE_SIZE>;

    fn add(self, rhs: Numeric<CORE_SIZE>) -> Numeric<CORE_SIZE> {
        Numeric::new(self.value + rhs.value)
    }
}

impl<const CORE_SIZE: usize> AddAssign<usize> for Numeric<CORE_SIZE> {
    fn add_assign(&mut self, rhs: usize) {
        self.value = (self.value + rhs) % CORE_SIZE;
    }
}

impl<const CORE_SIZE: usize> Sub<Numeric<CORE_SIZE>> for Numeric<CORE_SIZE> {
    type Output = Numeric<CORE_SIZE>;

    fn sub(self, rhs: Numeric<CORE_SIZE>) -> Numeric<CORE_SIZE> {
        Numeric::new(self.value + CORE_SIZE - rhs.value)
    }
}

impl<const CORE_SIZE: usize> SubAssign<usize> for Numeric<CORE_SIZE> {
    fn sub_assign(&mut self, rhs: usize) {
        self.value = (self.value + CORE_SIZE - rhs) % CORE_SIZE;
    }
}

impl<const CORE_SIZE: usize> Mul<Numeric<CORE_SIZE>> for Numeric<CORE_SIZE> {
    type Output = Numeric<CORE_SIZE>;

    fn mul(self, rhs: Numeric<CORE_SIZE>) -> Numeric<CORE_SIZE> {
        Numeric::new(self.value * rhs.value)
    }
}

impl<const CORE_SIZE: usize> Div<Numeric<CORE_SIZE>> for Numeric<CORE_SIZE> {
    type Output = Numeric<CORE_SIZE>;

    fn div(self, rhs: Numeric<CORE_SIZE>) -> Numeric<CORE_SIZE> {
        Numeric::new(self.value / rhs.value)
    }
}

impl<const CORE_SIZE: usize> Rem<Numeric<CORE_SIZE>> for Numeric<CORE_SIZE> {
    type Output = Numeric<CORE_SIZE>;

    fn rem(self, rhs: Numeric<CORE_SIZE>) -> Numeric<CORE_SIZE> {
        Numeric::new(self.value % rhs.value)
    }
}

impl<const CORE_SIZE: usize> Add<usize> for Numeric<CORE_SIZE> {
    type Output = Numeric<CORE_SIZE>;

    fn add(self, _rhs: usize) -> Numeric<CORE_SIZE> {
        Numeric::new(self.value + _rhs)
    }
}
