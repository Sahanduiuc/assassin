mod assassin;
use assassin::simulation::Simulation;
use assassin::feeds::discount_option_data::DiscountOptionData;
use assassin::models::pmcc::PMCC;

static INPUT_FILE: &'static str = "/Users/billrobinson/Desktop/aapl_2013.csv";

fn main() {
	let base_feed = DiscountOptionData::new(INPUT_FILE);
	let test_model = PMCC::new();

	let mut simulation = Simulation::new(
		Box::new(test_model),
		Box::new(base_feed),
	);

	simulation.run();

	println!(
		"Ran simulation ({} ticks) in {:.2} seconds",
		simulation.ticks_processed(),
		simulation.total_run_time()
	);
}
