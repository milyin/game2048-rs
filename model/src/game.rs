use crate::field::Field;

pub struct Game {
    field: Field,
    score: usize,
}

impl Game {
    pub fn new(width: usize, height: usize) -> Self {
        let field = Field::new(width, height);
        let score = 0;
        Self { field, score }
    }
    pub fn field(&self) -> &Field {
        self.field
    }

    pub fn score(&self) -> usize {
        self.score
    }

    fn swipe

    pub fn undo(&mut self) -> bool {
        if self.field.can_undo() {
            self.field.undo();
        }
    }
}
