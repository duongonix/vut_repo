const ALPHABET: &[char] = &[
    'a', 'Z', '_', '0', '9', ' ', '\t', '\n', '\r', '(', ')', ':', ',', '.', '@', '$', '"', '\\',
    '#', '+', '-', '*', '/', '%', '=', '!', '<', '>', '?', '猫', 'ệ', '💥', '\0',
];

pub(crate) struct Generator {
    state: u64,
}

impl Generator {
    pub(crate) fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }
    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
    pub(crate) fn source(&mut self) -> String {
        let length = usize::try_from(self.next() % 512).unwrap_or(0);
        (0..length)
            .map(|_| ALPHABET[usize::try_from(self.next()).unwrap_or(0) % ALPHABET.len()])
            .collect()
    }
}
