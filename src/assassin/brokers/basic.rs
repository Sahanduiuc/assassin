use std::collections::HashMap;

use assassin::order::Order;
use assassin::position::Position;
use assassin::quote::Quote;
use assassin::tick::Tick;
use assassin::traits::*;

extern crate chrono;

use self::chrono::prelude::*;

pub struct BasicBroker {
	balance: f64,
	open_positions: HashMap<String, Position>,
	orders: Vec<Order>,
	commission_schedule: Box<Commission>,
	commission_paid: f64,
	data_feed: Box<DataFeed>,
	quotes: HashMap<String, Quote>,
	current_date: DateTime<FixedOffset>,
}

impl BasicBroker {
	pub fn new(initial_balance: f64,
			commission_schedule: Box<Commission>,
			data_feed: Box<DataFeed>,
		) -> BasicBroker {

		if initial_balance <= 0.0 {
			panic!("balance must be > 0.0 (got {})", initial_balance);
		}

		// this is just so we have a default value
		let current_date = FixedOffset::east(0).ymd(2000, 1, 1).and_hms_milli(0, 0, 0, 0);

		BasicBroker{
			balance: initial_balance,
			open_positions: HashMap::new(),
			orders: vec![],
			commission_schedule: commission_schedule,
			commission_paid: 0.0,
			data_feed: data_feed,
			quotes: HashMap::new(),
			current_date: current_date,
		}
	}
}

impl Broker for BasicBroker {
	fn account_balance(&self) -> f64 {
		self.balance
	}

	fn quotes_for(&self, symbol: String) -> Vec<Quote> {
		let quotes: Vec<Quote> = vec![];

		quotes
	}

	// TODO: positions have a correct cost basis

	fn process_order(&mut self, order: Order) -> bool {
		// ensure enough cash available
		if order.cost_basis() > self.balance {
			println!(
				"not enough money (need {}, have {})",
				order.cost_basis(),
				self.balance
			);
			return false;
		}

		// TODO: move this back to the top if orders get a "filled" status
		self.orders.push(order.clone());

		self.open_positions.entry(order.option_name()).or_insert(Position::new(&order)).apply_order(&order);

		// TODO: delete position if its quantity is now 0

		self.balance += order.canonical_cost_basis();

		// apply commission to balance and running total of paid commission
		let commish = self.commission_schedule.commission_for(order);

		// TODO: edge case... commission is not factor into available money before applying order
		self.balance -= commish;
		self.commission_paid += commish;

		true
	}

	fn open_positions(&self) -> Vec<Position> {
		let mut positions: Vec<Position> = vec![];

		for (_, value) in &self.open_positions {
			if value.quantity() != 0 {
				positions.push(value.clone());
			}
		}

		positions
	}

	fn total_order_count(&self) -> i32 {
		self.orders.len() as i32
	}

	fn commission_paid(&self) -> f64 {
		self.commission_paid
	}

	fn close_all_positions(&mut self) {
		// TODO: close all open positions at last price
	}

	fn next_tick(&mut self) -> Option<Tick> {
		if let Some(tick) = self.data_feed.next_tick() {
			self.quotes.insert(tick.name(), tick.quote());

			let date = tick.date().num_days_from_ce();
			let current_date = self.current_date.num_days_from_ce();

			// if day has changed:
			//   1. close any open positions which are now expired
			//   2. remove any expired quotes
			if date != current_date {
				let mut new_quotes = self.quotes.clone();
				let mut removed_entry_count = 0;
				let mut closed_position_count = 0;

				for (key, quote) in &self.quotes {
					let mut new_positions: HashMap<String, Position> = HashMap::new();

					for (option_name, position) in &self.open_positions {
						if position.expiration_date() == self.current_date {
							println!("closing position: {}", position.name());
							// TODO: close position
							closed_position_count += 1;
						} else {
							let new_position = position.clone();
							new_positions.insert(option_name.clone(), new_position);
						}
					}

					self.open_positions = new_positions;

					if quote.expiration_date() == self.current_date {
						new_quotes.remove(key);
						removed_entry_count += 1;
					}
				}

				println!(
					"closed {} positions for {}",
					closed_position_count,
					self.current_date
				);
				println!(
					"purged {} expired entries for {}",
					removed_entry_count,
					self.current_date,
				);

				self.quotes = new_quotes;
				self.current_date = tick.date();
			}
			Some(tick)
		} else {
			None
		}
	}
}