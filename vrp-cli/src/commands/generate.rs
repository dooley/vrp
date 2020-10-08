use super::*;
use std::io::BufReader;
use std::process;
use vrp_cli::extensions::generate::generate_problem;
use vrp_cli::extensions::import::serialize_hre_problem;
use vrp_pragmatic::format::problem::serialize_problem;
use vrp_pragmatic::format::FormatError;
use vrp_pragmatic::validation::ValidationContext;

pub const FORMAT_ARG_NAME: &str = "FORMAT";
pub const PROTOTYPES_ARG_NAME: &str = "prototypes";
pub const OUT_RESULT_ARG_NAME: &str = "out-result";
pub const JOBS_SIZE_ARG_NAME: &str = "jobs-size";
pub const VEHICLES_SIZE_ARG_NAME: &str = "vehicles-size";
pub const LOCATIONS_ARG_NAME: &str = "locations";
pub const AREA_SIZE_ARG_NAME: &str = "area-size";

pub fn get_generate_app<'a, 'b>() -> App<'a, 'b> {
    App::new("generate")
        .about("Provides the way to generate meaningful problems for testing")
        .arg(
            Arg::with_name(FORMAT_ARG_NAME)
                .help("Specifies input type")
                .required(true)
                .possible_values(&["pragmatic", "hre"])
                .index(1),
        )
        .arg(
            Arg::with_name(PROTOTYPES_ARG_NAME)
                .help("Sets input files which contains a VRP definition prototype")
                .short("p")
                .long(PROTOTYPES_ARG_NAME)
                .required(true)
                .takes_value(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name(OUT_RESULT_ARG_NAME)
                .help("Specifies path to the file for result output")
                .short("o")
                .long(OUT_RESULT_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(LOCATIONS_ARG_NAME)
                .help("Specifies path to the file with a list of job locations")
                .short("l")
                .long(LOCATIONS_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(JOBS_SIZE_ARG_NAME)
                .help("Amount of jobs in the plan of generated problem")
                .short("j")
                .long(JOBS_SIZE_ARG_NAME)
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(VEHICLES_SIZE_ARG_NAME)
                .help("Amount of vehicle types in the fleet of generated problem")
                .short("v")
                .long(VEHICLES_SIZE_ARG_NAME)
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(AREA_SIZE_ARG_NAME)
                .help("Half side size of job distribution bounding box. Center is calculated using prototype locations")
                .short("a")
                .long(AREA_SIZE_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
}

pub fn run_generate(matches: &ArgMatches) {
    let input_format = matches.value_of(FORMAT_ARG_NAME).unwrap();
    let input_files = matches
        .values_of(PROTOTYPES_ARG_NAME)
        .map(|paths: Values| paths.map(|path| BufReader::new(open_file(path, "input"))).collect::<Vec<_>>());
    let locations_file = matches.value_of(LOCATIONS_ARG_NAME).map(|path| BufReader::new(open_file(path, "locations")));

    let jobs_size = parse_int_value::<usize>(matches, JOBS_SIZE_ARG_NAME, "jobs size").unwrap();
    let vehicles_size = parse_int_value::<usize>(matches, VEHICLES_SIZE_ARG_NAME, "vehicles size").unwrap();
    let area_size = parse_float_value::<f64>(matches, AREA_SIZE_ARG_NAME, "area size");

    match generate_problem(input_format, input_files, locations_file, jobs_size, vehicles_size, area_size) {
        Ok(problem) => {
            if let Err(errors) = ValidationContext::new(&problem, None).validate() {
                eprintln!(
                    "Generated problem has some validation errors:\n{}",
                    FormatError::format_many(errors.as_slice(), "\n")
                );
            };

            let out_result = matches.value_of(OUT_RESULT_ARG_NAME).map(|path| create_file(path, "out result"));
            let out_buffer = create_write_buffer(out_result);

            let result = match input_format {
                "pragmatic" => serialize_problem(out_buffer, &problem),
                "hre" => serialize_hre_problem(out_buffer, &problem),
                _ => {
                    eprintln!("Unknown output format: '{}'", input_format);
                    process::exit(1);
                }
            };

            if let Err(err) = result {
                eprintln!("Cannot serialize result problem: '{}'", err);
                process::exit(1);
            } else {
                println!("Problem has been generated");
            }
        }
        Err(err) => {
            eprintln!("Cannot generate problem: '{}'", err);
            process::exit(1);
        }
    }
}
