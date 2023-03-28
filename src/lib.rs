//! Scaphandre is an extensible monitoring agent for energy consumption metrics.
//!
//! It gathers energy consumption data from the system or other data sources thanks to components called *sensors*.
//!
//! Final monitoring data is sent to or exposed for monitoring tools thanks to *exporters*.
#[macro_use]
extern crate log;
pub mod exporters;
pub mod sensors;

#[cfg(target_os = "windows")]
use sensors::msr_rapl::MsrRAPLSensor;
#[cfg(target_os = "linux")]
use sensors::powercap_rapl::PowercapRAPLSensor;
use sensors::{Sensor, powercap_rapl};

use std::time::{Duration, SystemTime};

/// Create a new [`Sensor`] instance with the default sensor available,
/// with its default options.
pub fn get_default_sensor() -> impl Sensor {
    #[cfg(target_os = "linux")]
    return PowercapRAPLSensor::new(powercap_rapl::DEFAULT_BUFFER_PER_SOCKET_MAX_KBYTES, powercap_rapl::DEFAULT_BUFFER_PER_DOMAIN_MAX_KBYTES, false);

    #[cfg(target_os = "windows")]
    return MsrRAPLSensor::new();
}

/// Matches the sensor and exporter name and options requested from the command line and
/// creates the appropriate instances. Launchs the standardized entrypoint of
/// the choosen exporter: run()
/// This function should be updated to take new exporters into account.
// pub fn run(matches: ArgMatches) {
//     loggerv::init_with_verbosity(matches.occurrences_of("v")).unwrap();

//     let sensor_boxed = get_sensor(&matches);
//     let exporter_parameters;

//     let mut header = true;
//     if matches.is_present("no-header") {
//         header = false;
//     }

//     if let Some(stdout_exporter_parameters) = matches.subcommand_matches("stdout") {
//         if header {
//             scaphandre_header("stdout");
//         }
//         exporter_parameters = stdout_exporter_parameters.clone();
//         let mut exporter = StdoutExporter::new(sensor_boxed);
//         exporter.run(exporter_parameters);
//     } else if let Some(json_exporter_parameters) = matches.subcommand_matches("json") {
//         if header {
//             scaphandre_header("json");
//         }
//         exporter_parameters = json_exporter_parameters.clone();
//         let mut exporter = JsonExporter::new(sensor_boxed);
//         exporter.run(exporter_parameters);
//     } else if let Some(riemann_exporter_parameters) = matches.subcommand_matches("riemann") {
//         if header {
//             scaphandre_header("riemann");
//         }
//         exporter_parameters = riemann_exporter_parameters.clone();
//         let mut exporter = RiemannExporter::new(sensor_boxed);
//         exporter.run(exporter_parameters);
//     } else if let Some(prometheus_exporter_parameters) = matches.subcommand_matches("prometheus") {
//         if header {
//             scaphandre_header("prometheus");
//         }
//         exporter_parameters = prometheus_exporter_parameters.clone();
//         let mut exporter = PrometheusExporter::new(sensor_boxed);
//         exporter.run(exporter_parameters);
//     } else {
//         #[cfg(target_os = "linux")]
//         {
//             #[cfg(feature = "warpten")]
//             {
//                 if let Some(warp10_exporter_parameters) = matches.subcommand_matches("warp10") {
//                     if header {
//                         scaphandre_header("warp10");
//                     }
//                     exporter_parameters = warp10_exporter_parameters.clone();
//                     let mut exporter = Warp10Exporter::new(sensor_boxed);
//                     exporter.run(exporter_parameters);
//                 }
//             }
//             #[cfg(feature = "qemu")]
//             {
//                 if let Some(qemu_exporter_parameters) = matches.subcommand_matches("qemu") {
//                     if header {
//                         scaphandre_header("qemu");
//                     }
//                     exporter_parameters = qemu_exporter_parameters.clone();
//                     let mut exporter = QemuExporter::new(sensor_boxed);
//                     exporter.run(exporter_parameters);
//                 }
//                 error!("Warp10 exporter feature was not included in this build.");
//             }
//         }
//         error!("Couldn't determine which exporter to run.");
//     }
// }

fn current_system_time_since_epoch() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
}

/// Returns rust crate version, can be use used in language bindings to expose Rust core version
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

//  Copyright 2020 The scaphandre authors.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
