use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::Vector;
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, BorshStorageKey, PanicOnDefault, BlockHeight};

near_sdk::setup_alloc!(); // Memory init


const WIDTH: usize = 16;
const HEIGHT: usize = 16;


const FIELD_LEN: usize = (WIDTH / 8) * HEIGHT;


#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Board {
    pub field: Base64VecU8
}

impl Board { 
    pub fn new() -> Self {
        Self {
            field: Base64VecU8::from(vec![0u8; FIELD_LEN]),
        }
    }

    pub fn from(field: Base64VecU8) -> Self {
        assert_eq!(field.0.len(), FIELD_LEN);
        Self {
            field,
        }
    }

    pub fn is_bit_set(&self, x: usize, y: usize) -> bool {
        let index = y * WIDTH + x;
        let byte_index = index / 8;
        let bit_index = index & 7; // byte_index % 8
        ((self.field.0[byte_index] >> bit_index) & 1) != 0
    }

    pub fn set_bit(&mut self, x: usize, y: usize, bit: bool) {
        let index = y * WIDTH + x;
        let byte_index = index / 8;
        let bit_index = index & 7; // byte_index % 8
        self.field.0[byte_index] |= 1 << bit_index ;
        if !bit {
            self.field.0[byte_index] ^= 1u8 << bit_index;
        }
    }

    pub fn to_string(&self) -> Vec<String> {
        (0..HEIGHT)
            .map(|i| {
            (0..WIDTH)
                .map(|j| if self.is_bit_set(j, i) { 'X' } else { '.' })
                .collect()
        })
        .collect()
    }

    pub fn debug_logs(&self) {
        self.to_string()
            .into_iter()
            .for_each(|s| env::log(s.as_bytes()))
    }
}

// ----------

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct BoardWithBlock {
    pub board: Board,
    pub current_block_height: BlockHeight,
    pub prev_block_height: BlockHeight,
}

impl BoardWithBlock { 
    pub fn new(board: Board) -> Self {
        Self {
            board,
            current_block_height: env::block_index(),
            prev_block_height: 0,
        }
    }

    pub fn step(&self) -> BoardWithBlock {
        let board = &self.board;
        let mut new_board = Board::new();
        let block_height = env::block_index();

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let bit = board.is_bit_set(x, y);
                let mut sum = 0;
                for off_y in 0..=2 {
                    let ny = y + off_y;
                    for off_x in 0..=2 {
                        if off_x == 1 && off_y == 1 {
                            continue;
                        }
                        let nx = x + off_x;
                        if ny >= 1 && nx >= 1 && ny <= HEIGHT && nx <= WIDTH {
                            if board.is_bit_set(nx - 1, ny - 1) {
                                sum += 1;
                            }
                        }
                    }
                }
                if bit && sum == 2 || sum == 3 {
                    new_board.set_bit(x, y, true)
                }
            }
        }
        let prev_block_height = if block_height == self.current_block_height {
            self.prev_block_height
        } else {
            self.current_block_height
        };
        Self {
            board: new_board,
            current_block_height: block_height,
            prev_block_height,
        }

    }
}

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKey {
    Boards, //0x00
}



pub type BoardIndex = u64;

 
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub boards: Vector<BoardWithBlock>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            boards: Vector::new(StorageKey::Boards)
        }
    }

    pub fn create_board(&mut self, field: Base64VecU8) -> BoardIndex {
        let board = Board::from(field);
        board.debug_logs();
        let board_with_blocks = BoardWithBlock::new(board);
        let index = self.boards.len();
        self.boards.push(&board_with_blocks);
        index
    } 

    pub fn get_board(&self, index: BoardIndex) -> Option<BoardWithBlock> {
        let board = self.boards.get(index);
        if let Some(board) = board.as_ref() {
            board.board.debug_logs();
        }
        board
    }

    pub fn step(&mut self, index: BoardIndex) -> BoardWithBlock {
        env::log(b"Old board");
        let board = self.get_board(index).expect("No board");
        let new_board = board.step();
        self.boards.replace(index, &new_board);
        env::log(b"New board");
        new_board.board.debug_logs();
        new_board
    }
}


#[cfg(not(target_arch="wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    fn get_context(is_view: bool) -> VMContext{
        VMContextBuilder::new().is_view(is_view).build()
    }

    fn debug_board(board: &Board) {
        for i in 0..HEIGHT {
            for j in 0..WIDTH {
                let index = i * WIDTH + j;
                let byte_index = index / 8;
                let bit_index = index & 7; // byte_index % 8
                if ((board.field.0[byte_index] >> bit_index) & 1) != 0 {
                    print!("X");
                } else {
                    print!(".")
                }
            }
            println!();
        }
    }

    #[test]
    fn test_new() {
        let context = get_context(false);
        testing_env!(context);
    }

    #[test]
    fn test_board_create_get() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = Contract::new();

        let mut field = vec![0u8; FIELD_LEN];
        field[0] = 24;
        let index = contract.create_board(field.clone().into());

        assert_eq!(index, 0);

        testing_env!(get_context(true));

        let board = contract.get_board(0).unwrap();
        debug_board(&board.board);
        assert_eq!(board.board.field.0, field);
    }


    #[test]
    fn test_one_step() {
        let context = get_context(false);
        testing_env!(context.clone());
        let mut contract = Contract::new();

        let field = vec![0u8; FIELD_LEN];
        let mut init_board = Board::from(field.into());

        let str: String = near_sdk::serde_json::to_string(&init_board).unwrap();
        println!("{}", str);

        init_board.set_bit(4, 4, true);
        init_board.set_bit(5, 4, true);
        init_board.set_bit(6, 4, true);
        init_board.set_bit(6, 3, true);
        init_board.set_bit(6, 2, true);

        println!("Initial Board!");
        debug_board(&init_board);

        contract.create_board(init_board.field);

        for step in 0..10 {
            testing_env!(context.clone());

            let board = contract.step(0);
            println!("Step #{}", step);
            debug_board(&board.board);
        }

    }
}

