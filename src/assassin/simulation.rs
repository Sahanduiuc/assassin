use std::time::Instant;

use assassin::traits::*;

pub struct Simulation {
	feed: Box<DataFeed>,
	model: Box<Model>,
	broker: Box<Broker>,

	// TODO: add settings variables (slippage, spread multipliers, etc.)
	// TODO: add target stats that the model must hit (sharpe, DD, etc.)

	ticks_processed: i64,

	start_time: Instant,
}

impl Simulation {
	pub fn new(
		model: Box<Model>,
		feed: Box<DataFeed>,
		broker: Box<Broker>,
		) -> Simulation {

		Simulation {
			feed: feed,
			model: model,
			broker: broker,
			ticks_processed: 0,
			start_time: Instant::now(),
		}
	}

	pub fn run(&mut self) {
		self.model.before_simulation(&mut *self.broker);

		while let Some(tick) = self.feed.next_tick() {
			// TODO: maybe check that the ticks are in chronological order here?
			self.model.process_tick(tick, &mut *self.broker);
			self.ticks_processed += 1;
		}

		self.model.after_simulation(&mut *self.broker);
	}

	pub fn total_run_time(&self) -> f64 {
		let seconds = self.start_time.elapsed().as_secs() as f64;
		let nanoseconds = self.start_time.elapsed().subsec_nanos() as f64 * 1e-9;

		seconds + nanoseconds
	}

	pub fn ticks_processed(&self) -> i64 {
		self.ticks_processed
	}
}