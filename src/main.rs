mod assassin;
use assassin::simulation::Simulation;
use assassin::feeds::discount_option_data::DiscountOptionData;
use assassin::models::pmcc::PMCC;
use assassin::broker::Broker;
// use assassin::commission::null::NullCommission;
use assassin::commission::charles_schwab::CharlesSchwab;

static INPUT_FILE: &'static str = "/Users/billrobinson/Desktop/aapl_2013.csv";
static STARTING_CAPITAL: f32 = 100_000.0;

fn main() {
	let feed = DiscountOptionData::new(INPUT_FILE);
	let test_model = PMCC::new();

	let commission = CharlesSchwab::new();
	// let commission = NullCommission::new();
	let broker = Broker::new(STARTING_CAPITAL, Box::new(commission), Box::new(feed));

	let mut simulation = Simulation::new(Box::new(test_model), Box::new(broker));

	simulation.run();

	simulation.print_stats();
}
