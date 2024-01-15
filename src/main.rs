use std::collections::{BTreeMap, VecDeque, HashMap};
use uuid::Uuid;
use rand::Rng;

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
pub struct FillResult {
    // Orders filled (qty, price)
    pub filled_orders: Vec<(u64, u64)>, 
    pub remaining_qty: u64, 
    pub status: OrderStatus, 
}

impl FillResult {
    fn new() -> FillResult {
        FillResult {
            filled_orders: Vec::new(), 
            remaining_qty: u64::MAX, 
            status: OrderStatus::Uninitialized, 
        }
    }

    pub fn avg_fill_price(&self) -> f32 {
        let mut total_price_paid = 0;
        let mut total_qty = 0;
        for (q, p) in &self.filled_orders {
            total_price_paid += p * q;
            total_qty += q;
        }
        return total_price_paid as f32 / total_qty as f32;
    }
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
            price_levels: Vec::with_capacity(5000), // Pre-alloc
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
            order_loc: HashMap::with_capacity(5000), 
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

    pub fn add_limit_order(&mut self, s: Side, price: u64, order_qty: u64) -> FillResult {
        fn match_at_price_level(
            price_level: &mut VecDeque<Order>, 
            incoming_order_qty: &mut u64, 
            order_loc: &mut HashMap<String, (Side, usize)>,
        ) -> u64 {
            let mut done_qty = 0;
            for o in price_level.iter_mut() {
                if o.qty <= *incoming_order_qty {
                    done_qty += o.qty;
                    *incoming_order_qty -= o.qty;
                    o.qty = 0;
                    order_loc.remove(&o.order_id);
                } else {
                    o.qty -= *incoming_order_qty;
                    done_qty += *incoming_order_qty;
                    *incoming_order_qty = 0;
                }
            }

            price_level.retain(|x| x.qty != 0);
            done_qty
        }

        let mut remaining_order_qty = order_qty;
        print!("Got order with qty {}, at price {}\n", remaining_order_qty, price);

        let mut fill_result = FillResult::new();
        match s {
            Side::Bid => {
                let askbook = &mut self.ask_book;
                let price_map = &mut askbook.price_map;
                let price_levels = &mut askbook.price_levels;
                let mut price_map_iter = price_map.iter();

                if let Some((mut x, _)) = price_map_iter.next() {
                    while price >= *x {
                        let curr_level = price_map[x];
                        let matched_qty = match_at_price_level(
                            &mut price_levels[curr_level],
                            &mut remaining_order_qty,
                            &mut self.order_loc,
                        );

                        if matched_qty != 0 {
                            print!("Matched {} qty at price {}", matched_qty, x);
                            fill_result.filled_orders.push((matched_qty, *x));
                        }

                        if let Some((a, _)) = price_map_iter.next() {
                            x = a;
                        } else {
                            break;
                        }
                    }
                }
            }

            Side::Ask => {
                let bidbook = &mut self.bid_book;
                let price_map = &mut bidbook.price_map;
                let price_levels = &mut bidbook.price_levels;
                let mut price_map_iter = price_map.iter();

                if let Some((mut x, _)) = price_map_iter.next_back() {
                    while price <= *x {
                        let curr_level = price_map[x];
                        let matched_qty = match_at_price_level(
                            &mut price_levels[curr_level],
                            &mut remaining_order_qty,
                            &mut self.order_loc,
                        );
                        if matched_qty != 0 {
                            print!("Matched {} qty at price {}", matched_qty, x);
                            fill_result.filled_orders.push((matched_qty, *x));
                        }
                        if let Some((a, _)) = price_map_iter.next_back() {
                            x = a;
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        fill_result.remaining_qty = remaining_order_qty;
        if remaining_order_qty != 0 {
            print!("Still remaining qty {} at price level {}\n", remaining_order_qty, price);
            
            if remaining_order_qty == order_qty {
                fill_result.status = OrderStatus::Created;
            } else {
                fill_result.status = OrderStatus::PartiallyFilled;
            }

            self.create_new_limit_order(s, price, remaining_order_qty);

        } else {
            fill_result.status = OrderStatus::Filled;
        }

        self.update_bbo();

        fill_result
    }

    pub fn get_bbo(&self) {
        let total_bid_qty = self.bid_book.get_total_qty(self.best_bid_price);
        let total_ask_qty = self.ask_book.get_total_qty(self.best_ask_price);

        println!("Best bid {}, qty {}", self.best_bid_price, total_bid_qty);
        println!("Best ask {}, qty {}", self.best_ask_price, total_ask_qty);
        println!(
            "Spread is {:.6},",
            (self.best_ask_price - self.best_bid_price) as f32
        );
    }

}



fn main() {
    println!("Creating new Orderbook");
    let mut orderbook = OrderBook::new("AAPL".to_string());
    let mut rng = rand::thread_rng();
    for _ in 1..500 {
        orderbook.add_limit_order(Side::Bid, rng.gen_range(1..250), rng.gen_range(1..=500));
        orderbook.add_limit_order(Side::Ask, rng.gen_range(250..500), rng.gen_range(1..=500));
    }
    println!("Done!");
    orderbook.get_bbo();
    dbg!(orderbook);
}
