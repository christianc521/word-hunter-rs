/*
* Project: jobs-rs
* Desc: TUI application that solves for the largest words in a given Word Hunt matrix
* Dependencies: crossterm (terminal manipulation library)
*/
use crossterm::{
    cursor,
    event::{read, Event, KeyCode},
    terminal, ExecutableCommand, QueueableCommand,
};
use fxhash::FxBuildHasher;
use std::{
    char,
    collections::{HashMap, HashSet},
    fs,
    io::{stdout, Cursor, Write},
    str::FromStr,
    thread,
    time::Duration,
    usize,
};

type FxHashMap<K, V> = HashMap<K, V, FxBuildHasher>;

#[derive(Default)]
struct TrieNode {
    is_end_of_word: bool,
    children: FxHashMap<char, TrieNode>,
}

struct Trie {
    root: TrieNode,
}

impl Trie {
    fn new() -> Self {
        Trie {
            root: TrieNode::default(),
        }
    }

    fn insert(&mut self, word: &str) {
        let mut curr_node = &mut self.root;

        for c in word.chars() {
            curr_node = curr_node.children.entry(c).or_default();
        }
        curr_node.is_end_of_word = true;
    }

    fn contains(&self, word: &str) -> bool {
        let mut curr_node = &self.root;

        for c in word.chars() {
            match curr_node.children.get(&c) {
                Some(node) => curr_node = node,
                None => return false,
            }
        }

        curr_node.is_end_of_word
    }
}

struct Grid {
    letters: String,
    grid_array: [[(char, bool); 4]; 4],
    index: (u8, u8),
    curr_word: String,
}

impl Grid {
    fn new() -> Grid {
        Grid {
            letters: String::with_capacity(16),
            grid_array: [[(' ', false); 4]; 4],
            index: (0, 0),
            curr_word: String::from_str("").unwrap(),
        }
    }

    fn add(&mut self, c: char) {
        let len = self.letters.len();
        let row = len / 4;
        let col = len % 4;

        self.grid_array[row][col] = (c, false);
        self.letters.push(c);
    }

    fn delete(&mut self) {
        if !self.letters.is_empty() {
            self.letters.pop();

            let len = self.letters.len();
            let row = len / 4;
            let col = len % 4;
            self.grid_array[row][col] = (' ', false);
        }
    }

    fn get_char(&self, i: usize, j: usize) -> Option<char> {
        if i < 4 && j < 4 {
            Some(self.grid_array[i][j].0)
        } else {
            None
        }
    }
}

struct Words {
    dictionary_words: Vec<String>,
}

impl Words {
    fn new(file_path: &str) -> Self {
        let content = fs::read_to_string(file_path).expect("Failed to read file.");
        let words = content.lines().map(|line| line.to_string()).collect();

        Words {
            dictionary_words: words,
        }
    }
}

fn find_words(grid: &Grid, trie: &Trie) -> Vec<String> {
    let mut results = std::collections::HashSet::new();
    let mut visited = [[false; 4]; 4];

    for i in 0..4 {
        for j in 0..4 {
            let mut path = String::new();
            dfs(
                i,
                j,
                &mut path,
                &mut visited,
                grid,
                &trie.root,
                &mut results,
            );
        }
    }

    let mut sorted: Vec<String> = results.into_iter().collect();
    sorted.sort_unstable_by(|a, b| {
        let length_cmp = b.len().cmp(&a.len());
        length_cmp.then_with(|| a.cmp(b))
    });

    sorted
}

fn dfs(
    row: usize,
    col: usize,
    path: &mut String,
    visited: &mut [[bool; 4]; 4],
    grid: &Grid,
    trie: &TrieNode,
    found_words: &mut HashSet<String>,
) {
    if row >= 4 || col >= 4 || visited[row][col] {
        return;
    }

    let Some(c) = grid.get_char(row, col) else {
        return;
    };

    let Some(next_node) = trie.children.get(&c) else {
        return;
    };

    visited[row][col] = true;
    path.push(c);

    if next_node.is_end_of_word && path.len() >= 3 {
        found_words.insert(path.clone());
    }

    for dr in -1..=1 {
        for dc in -1..=1 {
            if dr == 0 && dc == 0 {
                continue;
            }

            let nr = row as i32 + dr;
            let nc = col as i32 + dc;

            if nr >= 0 && nr < 4 && nc >= 0 && nc < 4 {
                dfs(
                    nr as usize,
                    nc as usize,
                    path,
                    visited,
                    grid,
                    next_node,
                    found_words,
                );
            }
        }
    }

    visited[row][col] = false;
    path.pop();
}

fn main() -> std::io::Result<()> {
    // initial state
    let _ = terminal::enable_raw_mode().unwrap();
    let mut stdout = stdout();
    let mut quit = false;
    let mut found_words: Vec<String> = Vec::new();

    let mut grid = Grid::new();
    let words = Words::new("dictionary.txt");
    let mut trie = Trie::new();

    for word in words.dictionary_words {
        let str_word = word.as_str();
        trie.insert(str_word);
    }

    let (mut term_w, mut term_h) = terminal::size().unwrap();
    let divider_char = "─";
    let mut divider_row = divider_char.repeat(term_w as usize);

    stdout.execute(terminal::Clear(terminal::ClearType::All))?;

    // render loop
    while !quit {
        match read()? {
            Event::Key(event) => match event.code {
                KeyCode::Char(c) => {
                    if grid.letters.len() < grid.letters.capacity() {
                        grid.add(c);
                    }
                }
                KeyCode::Enter => {
                    found_words = find_words(&grid, &trie);
                    stdout.queue(cursor::MoveTo(0, 0)).unwrap();
                    for (idx, word) in found_words.iter().enumerate().take((term_h - 3) as usize) {
                        stdout.queue(cursor::MoveTo(0, idx as u16)).unwrap();
                        stdout.write(word.as_bytes()).unwrap();
                        stdout
                            .execute(terminal::Clear(terminal::ClearType::UntilNewLine))
                            .unwrap();
                    }
                }
                KeyCode::Backspace => grid.delete(),
                KeyCode::Esc => quit = true,
                _ => (),
            },
            Event::Resize(w, h) => {
                term_w = w;
                term_h = h;
                divider_row = divider_char.repeat(term_w as usize);
            }
            _ => todo!(),
        }
        // clear input line to account for deletes
        stdout.queue(cursor::MoveTo(0, term_h - 1)).unwrap();
        stdout.execute(terminal::Clear(terminal::ClearType::CurrentLine))?;

        // draw input divider
        stdout.queue(cursor::MoveTo(0, term_h - 2)).unwrap();
        stdout.write(divider_row.as_bytes()).unwrap();

        // draw letters
        stdout.queue(cursor::MoveTo(0, term_h - 1)).unwrap();
        stdout.write(grid.letters.as_bytes()).unwrap();
        stdout.flush()?;

        // render at 30fps
        thread::sleep(Duration::from_millis(33));
    }

    let _ = terminal::disable_raw_mode().unwrap();
    Ok(())
}
