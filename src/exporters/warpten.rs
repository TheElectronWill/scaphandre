use crate::exporters::*;
use crate::sensors::{RecordGenerator, Sensor, Topology};
use std::time::Duration;
use utils::get_scaphandre_version;

/// An exporter that sends power consumption data of the host and its processes to
/// a [Warp10](https://warp10.io) instance through **HTTP(s)**
/// (contributions welcome to support websockets).
pub struct Warp10Exporter {
    /// Sensors's topology, which gives access to the metrics
    topology: Topology,
    /// Warp10 client
    client: warp10::Client,
    /// Warp10 auth token
    write_token: String,

    step: Duration,
    qemu: bool,
}

/// Holds the arguments for a Warp10Exporter.
#[derive(clap::Args, Debug)]
pub struct ExporterArgs {
    /// FQDN or IP address of the Warp10 instance
    #[arg(short = 'H', long, default_value = "localhost")]
    pub host: String,

    /// TCP port of the Warp10 instance
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

    /// "http" or "https"
    #[arg(short, long, default_value = "http")]
    pub scheme: String,

    /// Auth token to write data to Warp10.
    /// If not specified, you must set the env variable SCAPH_WARP10_WRITE_TOKEN
    #[arg(short = 't', long)]
    pub write_token: Option<String>,

    /// Interval between two measurements, in seconds
    #[arg(short, long, value_name = "SECONDS", default_value_t = 2)]
    pub step: u64,

    /// Apply labels to metrics of processes looking like a Qemu/KVM virtual machine
    #[arg(short, long)]
    pub qemu: bool,
}

const TOKEN_ENV_VAR: &str = "SCAPH_WARP10_WRITE_TOKEN";

impl Exporter for Warp10Exporter {
    /// Control loop for self.iterate()
    fn run(&mut self) {
        loop {
            match self.iterate() {
                Ok(res) => debug!("Result: {:?}", res),
                Err(err) => error!("Failed ! {:?}", err),
            }
            std::thread::sleep(self.step);
        }
    }
    
    fn kind(&self) -> &str {
        "warp10"
    }
}

impl Warp10Exporter {
    /// Instantiates and returns a new Warp10Exporter
    pub fn new(sensor: &dyn Sensor, args: ExporterArgs) -> Warp10Exporter {
        // Prepare for measurement
        let topology = sensor
            .get_topology()
            .expect("sensor topology should be available");

        // Prepare for sending data to Warp10
        let scheme = args.scheme;
        let host = args.host;
        let port = args.port;
        let client = warp10::Client::new(&format!("{scheme}://{host}:{port}")).expect("warp10 Client could not be created");
        let write_token = args.write_token.unwrap_or_else(|| {
            std::env::var(TOKEN_ENV_VAR).expect(&format!("No token found, you must provide either --write-token or the env var {TOKEN_ENV_VAR}"))
        });

        Warp10Exporter {
            topology,
            client,
            write_token,
            step: Duration::from_secs(args.step),
            qemu: args.qemu,
        }
    }

    /// Collects data from the Topology, creates warp10::Data objects containing the
    /// metric itself and some labels attaches, stores them in a vector and sends it
    /// to Warp10
    pub fn iterate(
        &mut self,
    ) -> Result<Vec<warp10::Warp10Response>, warp10::Error> {
        let writer = self.client.get_writer(self.write_token.clone());
        self.topology
            .proc_tracker
            .clean_terminated_process_records_vectors();

        debug!("Refreshing topology.");
        self.topology.refresh();

        let records = self.topology.get_records_passive();
        let scaphandre_version = get_scaphandre_version();

        let labels = vec![];

        let mut data = vec![warp10::Data::new(
            time::OffsetDateTime::now_utc(),
            None,
            String::from("scaph_self_version"),
            labels.clone(),
            warp10::Value::Double(scaphandre_version.parse::<f64>().unwrap()),
        )];

        if let Some(metric_value) = self.topology.get_process_cpu_usage_percentage(
            IProcess::myself(&self.topology.proc_tracker).unwrap().pid,
        ) {
            data.push(warp10::Data::new(
                time::OffsetDateTime::now_utc(),
                None,
                String::from("scaph_self_cpu_usage_percent"),
                labels.clone(),
                warp10::Value::Int(metric_value.value.parse::<i32>().unwrap()),
            ));
        }

        if let Some(metric_value) = self.topology.get_process_cpu_usage_percentage(
            IProcess::myself(&self.topology.proc_tracker).unwrap().pid,
        ) {
            data.push(warp10::Data::new(
                time::OffsetDateTime::now_utc(),
                None,
                String::from("scaph_self_cpu_usage_percent"),
                labels.clone(),
                warp10::Value::Int(metric_value.value.parse::<i32>().unwrap()),
            ));
        }

        if let Ok(metric_value) = procfs::process::Process::myself().unwrap().statm() {
            let value = metric_value.size * procfs::page_size().unwrap() as u64;
            data.push(warp10::Data::new(
                time::OffsetDateTime::now_utc(),
                None,
                String::from("scaph_self_mem_total_program_size"),
                labels.clone(),
                warp10::Value::Int(value as i32),
            ));
            let value = metric_value.resident * procfs::page_size().unwrap() as u64;
            data.push(warp10::Data::new(
                time::OffsetDateTime::now_utc(),
                None,
                String::from("scaph_self_mem_resident_set_size"),
                labels.clone(),
                warp10::Value::Int(value as i32),
            ));
            let value = metric_value.shared * procfs::page_size().unwrap() as u64;
            data.push(warp10::Data::new(
                time::OffsetDateTime::now_utc(),
                None,
                String::from("scaph_self_mem_shared_resident_size"),
                labels.clone(),
                warp10::Value::Int(value as i32),
            ));
        }

        let metric_value = self.topology.stat_buffer.len();
        data.push(warp10::Data::new(
            time::OffsetDateTime::now_utc(),
            None,
            String::from("scaph_self_topo_stats_nb"),
            labels.clone(),
            warp10::Value::Int(metric_value as i32),
        ));

        let metric_value = self.topology.record_buffer.len();
        data.push(warp10::Data::new(
            time::OffsetDateTime::now_utc(),
            None,
            String::from("scaph_self_topo_records_nb"),
            labels.clone(),
            warp10::Value::Int(metric_value as i32),
        ));

        let metric_value = self.topology.proc_tracker.procs.len();
        data.push(warp10::Data::new(
            time::OffsetDateTime::now_utc(),
            None,
            String::from("scaph_self_topo_procs_nb"),
            labels.clone(),
            warp10::Value::Int(metric_value as i32),
        ));

        for socket in &self.topology.sockets {
            let mut metric_labels = labels.clone();
            metric_labels.push(warp10::Label::new("socket_id", &socket.id.to_string()));
            let metric_value = socket.stat_buffer.len();
            data.push(warp10::Data::new(
                time::OffsetDateTime::now_utc(),
                None,
                String::from("scaph_self_socket_stats_nb"),
                metric_labels.clone(),
                warp10::Value::Int(metric_value as i32),
            ));
            let metric_value = socket.record_buffer.len();
            data.push(warp10::Data::new(
                time::OffsetDateTime::now_utc(),
                None,
                String::from("scaph_self_socket_records_nb"),
                metric_labels.clone(),
                warp10::Value::Int(metric_value as i32),
            ));

            let socket_records = socket.get_records_passive();
            if !socket_records.is_empty() {
                let socket_energy_microjoules = &socket_records.last().unwrap().value;
                if let Ok(metric_value) = socket_energy_microjoules.parse::<i64>() {
                    data.push(warp10::Data::new(
                        time::OffsetDateTime::now_utc(),
                        None,
                        String::from("scaph_socket_energy_microjoules"),
                        metric_labels.clone(),
                        warp10::Value::Long(metric_value),
                    ));
                }

                if let Some(metric_value) = socket.get_records_diff_power_microwatts() {
                    data.push(warp10::Data::new(
                        time::OffsetDateTime::now_utc(),
                        None,
                        String::from("scaph_socket_power_microwatts"),
                        metric_labels.clone(),
                        warp10::Value::Long(metric_value.value.parse::<i64>().unwrap()),
                    ));
                }
            }

            for domain in &socket.domains {
                let mut metric_labels = labels.clone();
                metric_labels.push(warp10::Label::new("rapl_domain_name", &domain.name));
                let metric_value = domain.record_buffer.len();
                data.push(warp10::Data::new(
                    time::OffsetDateTime::now_utc(),
                    None,
                    String::from("scaph_self_domain_records_nb"),
                    metric_labels.clone(),
                    warp10::Value::Int(metric_value as i32),
                ));
            }
        }

        if !records.is_empty() {
            let record = records.last().unwrap();
            let metric_value = record.value.clone();

            data.push(warp10::Data::new(
                time::OffsetDateTime::now_utc(),
                None,
                String::from("scaph_host_energy_microjoules"),
                labels.clone(),
                warp10::Value::Long(metric_value.parse::<i64>().unwrap()),
            ));

            if let Some(metric_value) = self.topology.get_records_diff_power_microwatts() {
                data.push(warp10::Data::new(
                    time::OffsetDateTime::now_utc(),
                    None,
                    String::from("scaph_host_power_microwatts"),
                    labels.clone(),
                    warp10::Value::Long(metric_value.value.parse::<i64>().unwrap()),
                ));
            }
        }

        let res = writer.post_sync(data)?;

        let mut results = vec![res];

        let mut process_data = vec![warp10::Data::new(
            time::OffsetDateTime::now_utc(),
            None,
            String::from("scaph_self_version"),
            labels.clone(),
            warp10::Value::Double(scaphandre_version.parse::<f64>().unwrap()),
        )];

        let processes_tracker = &self.topology.proc_tracker;
        for pid in processes_tracker.get_alive_pids() {
            let exe = processes_tracker.get_process_name(pid);
            let cmdline = processes_tracker.get_process_cmdline(pid);

            let mut plabels = labels.clone();
            plabels.push(warp10::Label::new("pid", &pid.to_string()));
            plabels.push(warp10::Label::new("exe", &exe));
            if let Some(cmdline_str) = cmdline {
                if self.qemu {
                    if let Some(vmname) = utils::filter_qemu_cmdline(&cmdline_str) {
                        plabels.push(warp10::Label::new("vmname", &vmname));
                    }
                }
                plabels.push(warp10::Label::new(
                    "cmdline",
                    &cmdline_str.replace('\"', "\\\""),
                ));
            }
            let metric_name = format!(
                "{}_{}_{}",
                "scaph_process_power_consumption_microwats", pid, exe
            );
            if let Some(power) = self.topology.get_process_power_consumption_microwatts(pid) {
                process_data.push(warp10::Data::new(
                    time::OffsetDateTime::now_utc(),
                    None,
                    metric_name,
                    plabels,
                    warp10::Value::Long(power.value.parse::<i64>().unwrap()),
                ));
            }
        }
        let process_res = writer.post_sync(process_data)?;

        //if let Some(token) = read_token {
        //let reader = client.get_reader(token.to_owned());
        //let parameters = warp10::data::ParameterSet::new(
        //"scaph_host_power_microwatts{}".to_string(),
        //Format::Text,
        //None, None, None,
        //Some(String::from("now")), Some(String::from("-10")),
        //None, None, None
        //);
        //let response = reader.get_sync(parameters);
        //match response {
        //Ok(resp) => warn!("response is: {:?}", resp),
        //Err(err) => panic!("error is: {:?}", err)
        //}
        //}

        results.push(process_res);

        Ok(results)
    }
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
