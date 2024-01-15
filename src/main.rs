use std::collections::{BTreeMap, VecDeque, HashMap};
use uuid::Uuid;

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
    pub order_id: String, 
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
    order_loc: HashMap<String, (Side, usize)>,
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

    pub fn cancel_order(&mut self, order_id: String) -> Result<String, &str> {
        if let Some((side, price_level)) = self.order_loc.get(&order_id) {
            let curr_price_deq = match side {
                Side::Ask => self.ask_book.price_levels.get_mut(*price_level).unwrap(), 
                Side::Bid => self.bid_book.price_levels.get_mut(*price_level).unwrap(), 
            };
            curr_price_deq.retain(|x| x.order_id != order_id);
            self.order_loc.remove(&order_id);
            let message = format!("Successfully cancelled order {}!", order_id);
            Ok(message)
        } else {
            Err("No valid order id!")
        }
    }

    pub fn create_new_limit_order(&mut self, s: Side, price: u64, qty: u64) -> String {
        let order_id: String = Uuid::new_v4().to_string();
        let book = match s {
            Side::Ask => &mut self.ask_book, 
            Side::Bid => &mut self.bid_book, 
        };
        let order = Order { order_id: order_id.clone(), qty };

        if let Some(price_level_idx) = book.price_map.get(&price) {
            book.price_levels[*price_level_idx].push_back(order);
            self.order_loc.insert(order_id.clone(), (s, *price_level_idx));
        } else {
            let new_loc = book.price_levels.len();
            book.price_map.insert(price, new_loc);
            let mut vec_deq = VecDeque::new();
            vec_deq.push_back(order);
            book.price_levels.push(vec_deq);
            self.order_loc.insert(order_id.clone(), (s, new_loc));
        }

        order_id
    }

    // Using BTreeMap so time complexity is O(n), consider using vectors
    fn update_bbo(&mut self) {
        for (p, u) in self.bid_book.price_map.iter().rev() {
            if !self.bid_book.price_levels[*u].is_empty() {
                self.best_bid_price = *p;
                break;
            }
        }

        for (p, u) in self.ask_book.price_map.iter() {
            if !self.ask_book.price_levels[*u].is_empty() {
                self.best_ask_price = *p;
                break;
            }
        }
    }
}



fn main() {
    println!("Hello, world!");
}
