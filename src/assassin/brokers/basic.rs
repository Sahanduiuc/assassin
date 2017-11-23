use std::collections::HashMap;

use assassin::order::Order;
use assassin::position::Position;
use assassin::quote::Quote;
use assassin::traits::*;
use assassin::util::*;

extern crate chrono;

use self::chrono::prelude::*;

pub struct BasicBroker {
	balance: f64,
	positions: HashMap<String, Position>,
	orders: Vec<Order>,
	commission_schedule: Box<Commission>,
	commission_paid: f64,
	data_feed: Box<DataFeed>,
	// TODO: convert this into a HashMap<String, HashMap<String,Quote>>
	quotes: HashMap<String, Quote>,
	current_date: DateTime<Utc>,
	ticks_processed: i64,
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
		let current_date = Utc::now();

		BasicBroker{
			balance: initial_balance,
			positions: HashMap::new(),
			orders: vec![],
			commission_schedule: commission_schedule,
			commission_paid: 0.0,
			data_feed: data_feed,
			quotes: HashMap::new(),
			current_date: current_date,
			ticks_processed: 0,
		}
	}
}

impl Broker for BasicBroker {
	fn orders(&self) -> Vec<Order> {
		self.orders.clone()
	}

	fn process_simulation_data(&mut self, model: &mut Model) {
		let mut day_changed;

		// manually consume the first tick here so we don't have to check
		// to see if it's the first tick every single time
		{
			let first_tick = self.data_feed.next_tick().unwrap();
			self.current_date = first_tick.date();
			self.quotes.insert(first_tick.name(), first_tick.quote());
		}

		while let Some(tick) = self.data_feed.next_tick() {
			day_changed = tick.date() != self.current_date;

			// ----- trading day logic -----------------------------------------

			// with EOD data, we run every time the day changes
			if day_changed {
				// TODO: convert this to a channel send with a timeout
				model.run_logic(self);
			}

			// ----- after hours cleanup ---------------------------------------

			self.current_date = tick.date();

			if day_changed {
				// force close anything that is expiring and that the model
				// didn't already close the last trading day.  do this before
				// we reset the quotes so that the last trading day's quotes
				// are used when closing positions.
				self.close_expired_positions();

				self.quotes = HashMap::new();
			}

			// ----- next day --------------------------------------------------

			// TODO: maybe check that the ticks are in chronological order here?
			// TODO: record last_tick time on struct

			// update quote for this option
			self.quotes.insert(tick.name(), tick.quote());

			self.ticks_processed += 1;
		}

		self.close_all_positions();
	}

	fn close_expired_positions(&mut self) {
		if self.positions.is_empty() {
			return;
		}

		let mut orders = vec![];

		for (option_name, position) in &self.positions {
			if position.is_open() && position.is_expired(self.current_date) {

				println!("closing position {} due to expiration:", option_name);

				let quote = self.quote_for(position.name()).unwrap();
				let quantity = position.quantity();
				let action;
				let price;

				// TODO: something better than filling at the worst possible price?

				let order = if position.is_long() {
					action = "sell";
					price = quote.bid();

					Order::new_sell_close_order(&quote, quantity, price)
				} else {
					action = "buy";
					price = quote.ask();

					Order::new_buy_close_order(&quote, quantity, price)
				};

				let commish = self.commission_schedule.commission_for(&order);
				let total = order.canonical_cost_basis() + commish;

				println!(
					"  {}ing contracts @ {} + {} commission ({} total)",
					action,
					format_money(price),
					format_money(commish),
					format_money(total),
				);

				orders.push(order);
			}
		}

		for order in orders {
			if ! self.process_order(order) {
				panic!("failed to process_order... margin call?");
			}
		}
	}

	fn ticks_processed(&self) -> i64 {
		self.ticks_processed
	}

	fn current_date(&self) -> DateTime<Utc> {
		self.current_date
	}

	fn account_balance(&self) -> f64 {
		self.balance
	}

	fn quote_for(&self, option_name: &str) -> Option<Quote> {
		match self.quotes.get(option_name) {
			Some(q) => Some(q.clone()),
			None    => None,
		}
	}

	// TODO: this should only return quotes for the desired symbol
	fn quotes_for(&self, _symbol: &str) -> Vec<Quote> {
		self.quotes.iter().map(|(_, q)| q.clone()).collect()
	}

	// TODO: positions have a correct cost basis

	fn process_order(&mut self, order: Order) -> bool {

		// TODO: assign a unique id to each order

		println!("Order received: {}", order.summary());

		// TODO: ensure that days remaining is > 0
		//       since we only buy at end of day, if there are no days left
		//       the the contract is _already_ expired.

		// TODO: validate that the option_name in the order actually exists

		let commish = self.commission_schedule.commission_for(&order);

		// TODO: only check cash for _buy_ orders

		if order.is_buy() {
			// ensure enough cash available
			if order.cost_basis() + commish > self.balance {
				println!(
					"not enough money (need {} + {} commission, have {})",
					format_money(order.cost_basis()),
					format_money(commish),
					format_money(self.balance),
				);
				return false;
			}
		}

		// TODO: check buying power instead of just cash

		let filled_order = &mut order.clone();

		// fill the order and record it
		filled_order.filled_at(order.limit(), commish);
		self.orders.push(filled_order.clone());

		// TODO: replace this .to_string() with &str support
		let key = filled_order.option_name().to_string();

		self.positions.entry(key).or_insert(Position::new(&filled_order)).apply_order(&filled_order);

		let original_balance = self.balance;

		// TODO: put this stuff in an apply_order() function or something
		{
			self.balance += filled_order.canonical_cost_basis();

			// apply commission to balance and running total of paid commission
			self.balance -= commish;
			self.commission_paid += commish;
		}

		println!(
			"ORDER FILLED. Commission: {} - Old balance: {} - New balance: {}",
			format_money(commish),
			format_money(original_balance),
			format_money(self.balance),
		);

		true
	}

	fn positions(&self) -> Vec<Position> {
		self.positions.iter().map(|(_, p)| p.clone()).collect()
	}

	fn open_positions(&self) -> Vec<Position> {
		self.positions().into_iter().filter(|p| p.is_open() ).collect()
	}

	fn total_order_count(&self) -> i32 {
		self.orders.len() as i32
	}

	fn commission_paid(&self) -> f64 {
		self.commission_paid
	}

	fn close_all_positions(&mut self) {
		println!("TODO: close all open positions at last price");
		println!("");
	}
}