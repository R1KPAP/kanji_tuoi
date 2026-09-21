pub struct History {
    pub items: Vec<char>,
    pub cursor: usize,
}

impl History {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            cursor: 0,
        }
    }

    pub fn current(&self) -> char {
        self.items[self.cursor]
    }

    pub fn push(&mut self, next: char) -> bool {
        if !self.items.is_empty() && self.current() == next {
            return false;
        }
        self.items.truncate(self.cursor + 1);
        self.items.push(next);
        self.cursor = self.items.len() - 1;
        return true;
    }

    pub fn back(&mut self) -> bool {
        if self.cursor > 0 {
            self.cursor -= 1;
            true
        } else if !self.items.is_empty() {
            self.items = Vec::new();
            true
        } else {
            false
        }
    }

    pub fn forward(&mut self) -> bool {
        if self.cursor + 1 < self.items.len() {
            self.cursor += 1;
            true
        } else {
            false
        }
    }
}