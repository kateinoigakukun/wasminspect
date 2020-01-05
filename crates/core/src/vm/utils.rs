pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L: Copy, R: Copy> Copy for Either<L, R> { }
impl<L: Clone, R: Clone> Clone for Either<L, R> {
    fn clone(&self) -> Either<L, R> {
        match self {
            Self::Left(l) => Self::Left(l.clone()),
            Self::Right(r) => Self::Right(r.clone())
        }
    }
 }