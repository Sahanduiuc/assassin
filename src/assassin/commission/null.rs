use assassin::traits::*;
use assassin::order::Order;

pub struct NullCommission {}

impl NullCommission {
	pub fn new() -> NullCommission {
		NullCommission{}
	}
}

impl Commission for NullCommission {
	fn commission_for(&self, _order: &Order) -> f32 {
		0.0
	}
}