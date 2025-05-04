/// Iterator over file size in chunks
/// 
pub struct Chunk {
    from: u64,
    to: u64,
    part: i32,
    size: u64,
    chunk_size: u64,
}

impl Chunk {
    /// Creates a new chunk instance 
    /// 
    /// # Arguments
    /// 
    /// * 'size' - size of file to be chunked
    /// * 'chunk_size' - size of each chunk
    pub fn new(size: u64, chunk_size: u64) -> Self {
        Chunk {
            from: 0,
            to: 0,
            part: 1,
            size,
            chunk_size,
        }
    }
}

/// Implementation of the Iterator trait
/// 
impl Iterator for Chunk {
    type Item = (i32, u64, u64);

    fn next(&mut self) -> Option<Self::Item> {
        if self.to >= self.size - 1 { return None; }

        self.to = self.from + self.chunk_size - 1;
        if self.to > self.size - 1 {
            self.to = self.size - 1;
        }

        let result = Some((self.part, self.from, self.to));

        self.from = self.to + 1;
        self.part += 1;

        result
    }
}
