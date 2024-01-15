use std::collections::{BTreeMap, VecDeque, HashMap};

#[derive(Debug)]
pub enum Side {
    Ask, 
    Bid
}

#[derive(Debug)]
pub enum OrderStatus {
    Uninitialized, 
    Created, 
    Filled, 
    PartiallyFilled, 
}

#[derive(Debug)]
pub struct Order {
    pub order_id: u64, 
    pub qty: u64, 
}

#[derive(Debug)]
struct HalfBook {
    s: Side, 
    price_map: BTreeMap<u64, usize>, 
    price_levels: Vec<VecDeque<Order>>, 
}

impl HalfBook {
    pub fn new(s: Side) -> HalfBook {
        HalfBook {
            s, 
            price_map: BTreeMap::new(), 
            price_levels: Vec::with_capacity(50_000), // Pre-alloc
        }
    }

    pub fn get_total_qty(&self, price: u64) -> u64 {
        self.price_levels[self.price_map[&price]]
            .iter()
            .map(|s| s.qty)
            .sum()
    }
}

#[derive(Debug)]
pub struct OrderBook {
    symbol: String, 
    best_ask_price: u64, 
    best_bid_price: u64, 
    ask_book: HalfBook,
    bid_book: HalfBook,
     // for fast cancel, id -> (side, price_level)
    order_loc: HashMap<u64, (Side, usize)>,
}

impl OrderBook {
    pub fn new(symbol: String) -> OrderBook {
        OrderBook {
            symbol, 
            best_ask_price: u64::MAX, 
            best_bid_price: u64::MIN, 
            bid_book: HalfBook::new(Side::Bid), 
            ask_book: HalfBook::new(Side::Ask), 
            order_loc: HashMap::with_capacity(50_000), 
        }
    }
}



fn main() {
    println!("Hello, world!");
}
