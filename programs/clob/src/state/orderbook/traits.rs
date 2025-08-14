use super::order::{Fill, Order};
use anchor_lang::prelude::*;

// Abstract OrderBook trait for different implementations
pub trait OrderBook {
    fn insert_order(&mut self, order: Order) -> Result<()>;
    fn remove_order(&mut self, order_id: u64) -> Result<Option<Order>>;
    fn get_best_price(&self) -> Option<u64>;
    fn match_orders(&mut self, incoming_order: &mut Order) -> Result<Vec<Fill>>;
    fn find_order_by_id(&self, order_id: u64) -> Option<Order>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
}
