use curl::easy::Easy;
use std::{
    thread::{self, JoinHandle}, 
    env, 
    io::{Write, Read}, 
    fs::OpenOptions,
    time::Duration, 
    fmt, 
    sync::{Arc, atomic::{AtomicBool, Ordering}}
};

#[cfg(target_family = "unix")]
use rustc_hash::FxHashMap;
#[cfg(target_family = "unix")]
use sysinfo::{System, SystemExt, NetworkExt};

const COLLECT_URL: &str = "https://origin.speedtestdemon.com/collect.php";

fn main() -> Result<(), anyhow::Error> {
    let using_vpn = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicBool::new(false));

    let using_vpn_clone = using_vpn.clone();
    let finished_clone = finished.clone();


    let vpn_check_thread = thread::spawn(move || vpn_check(&using_vpn_clone, &finished_clone));

    let mut urls: Vec<String> = env::args().collect();
    
    let mut thread_handles: Vec<JoinHandle<_>> = vec![];

    urls.remove(0);

    for url in urls {
        let using_vpn_clone = using_vpn.clone();
        let handle = thread::spawn(move || benchmark(url, &using_vpn_clone));
        thread_handles.push(handle);
        thread::sleep(Duration::from_secs(60));
    }

    for handle in thread_handles {
        handle.join().unwrap().unwrap();
    }

    finished.store(true, Ordering::Relaxed);

    vpn_check_thread.join().unwrap();

    Ok(())
}

fn benchmark(url: impl AsRef<str>, using_vpn: &AtomicBool) -> Result<(), anyhow::Error> {
    let url = url.as_ref();
    // The vector that'll contain all results i.e cold, hot and warm
    let mut end_results: Vec<String> = vec![];

    let cold_cache = make_request(url)?;

    let cold_cache_result = format!(
        "\n{}\n-------------------- Cold Cache --------------------\n{}\n-------------------- Cold Cache End --------------------\n", 
        url,
        cold_cache
    );

    println!("{}", cold_cache_result);

    end_results.push(cold_cache_result);

    drop(cold_cache);

    let mut results_vec: Vec<CurlResult> = vec![];

    let n = 10;

    for _ in 0..n {
        let result = make_request(url)?;

        results_vec.push(result);
    }

    let mut hot_cache = results_vec.remove(0);

    for result in results_vec {
        hot_cache = hot_cache + result;
    }

    hot_cache.namelookup_time /= n;
    hot_cache.connect_time /= n;
    hot_cache.appconnect_time /= n;
    hot_cache.pretransfer_time /= n;
    hot_cache.redirect_time /= n;
    hot_cache.starttransfer_time /= n;
    hot_cache.download_time /= n;
    hot_cache.total_time /= n;

    let hot_cache_result = format!(
        "\n{}\n-------------------- Hot Cache --------------------\n{}\n-------------------- Hot Cache End --------------------", 
        url,
        hot_cache
    );

    println!("{}", hot_cache_result);

    end_results.push(hot_cache_result);

    drop(hot_cache);

    let mut minutes: u8 = 0;

    println!("Sleeping for 30 minutes\n");

    while minutes < 30 {
        if using_vpn.load(Ordering::Relaxed) {
            return Ok(());
        }
        // Wait for half an hour for cache to get warm
        thread::sleep(Duration::from_secs(60));
        minutes += 1;
    }

    let warm_cache = make_request(url)?;

    let warm_cache_result = format!(
        "\n{}\n-------------------- Warm Cache --------------------\n{}\n-------------------- Warm Cache End --------------------", 
        url,
        warm_cache
    );

    end_results.push(warm_cache_result);

    drop(warm_cache);

    let request_body = end_results.join("\n");
    
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("results.txt")?;

    let mut bytes = request_body.as_bytes();
    
    file.write(bytes)?;

    let mut handle = Easy::new();

    handle.url(COLLECT_URL)?;

    handle.post(true)?;

    let mut transfer = handle.transfer();

    transfer.read_function(|into| Ok(bytes.read(into).unwrap()))?;

    transfer.perform()?;

    println!("Benchmarked {} successfully.", url);

    Ok(())
}

#[cfg(target_family = "unix")]
fn vpn_check(atomic_bool: &AtomicBool, finished: &AtomicBool) {
    let mut system = System::new();
    let mut track_map: FxHashMap<String, (u64, u64)> = FxHashMap::default();

    while !finished.load(Ordering::Relaxed) {
        if atomic_bool.load(Ordering::Relaxed) {
            println!("VPN Detected.");
            break;
        }
        thread::sleep(Duration::from_millis(500));
        system.refresh_networks();
        for (interface, network) in system.networks() {
            if !interface.contains("tun") {
                continue;
            }

            let (total_received, total_transmitted) = match track_map.get(interface) {
                Some(tuple) => *tuple,
                None => {
                    track_map.insert(interface.to_owned(), (network.total_received(), network.total_transmitted()));
                    continue;
                }
            };

            if total_received != network.total_received() || total_transmitted != network.total_transmitted() {
                atomic_bool.store(true, Ordering::Relaxed);
                break;
            }

        }

    }
}

#[cfg(target_family = "windows")]
fn vpn_check(atomic_bool: &AtomicBool, _: &AtomicBool){
    for adapter in ipconfig::get_adapters().expect("Couldn't get adapters") {
        if adapter.if_type() == ipconfig::IfType::Unsupported || adapter.if_type() == ipconfig::IfType::Ppp{
            if adapter.oper_status() == ipconfig::OperStatus::IfOperStatusUp {
                println!("-------------- VPN Detected --------------");
                atomic_bool.store(true, Ordering::Relaxed);
                break;
            }
        }
    }
}

struct CurlResult {
    pub headers: String,
    pub namelookup_time: Duration,
    pub connect_time: Duration,
    pub appconnect_time: Duration,
    pub pretransfer_time: Duration,
    pub redirect_time: Duration,
    pub starttransfer_time: Duration,
    pub total_time: Duration,
    pub download_time: Duration,
    pub speed: usize,
    pub bytes: Vec<u8>
}

impl CurlResult {
    // Transform cumulative seconds into individual seconds
    pub fn normalize(&mut self){
        self.download_time -= self.starttransfer_time;

        if self.starttransfer_time.as_nanos() > 0{
            self.starttransfer_time -= self.pretransfer_time;
        }
        if self.redirect_time.as_nanos() > 0{
            self.redirect_time -= self.starttransfer_time;
        }
        if self.pretransfer_time.as_nanos() > 0 {
            self.pretransfer_time -= self.appconnect_time;
        }
        if self.appconnect_time.as_nanos() > 0 {
            self.appconnect_time -= self.connect_time;
        }
        if self.connect_time.as_nanos() > 0 {
            self.connect_time -= self.namelookup_time; 
        }
    }
}

impl std::ops::Add for CurlResult {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            headers: self.headers,
            namelookup_time: self.namelookup_time + other.namelookup_time,
            connect_time: self.connect_time + other.connect_time,
            appconnect_time: self.appconnect_time + other.appconnect_time,
            pretransfer_time: self.pretransfer_time + other.pretransfer_time,
            redirect_time: self.redirect_time + other.redirect_time,
            starttransfer_time: self.starttransfer_time + other.starttransfer_time,
            total_time: self.total_time + other.total_time,
            download_time: self.download_time + other.download_time,
            speed: self.speed + other.speed,
            bytes: self.bytes
        }
    }
}
impl fmt::Display for CurlResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\nName Lookup Time: {}ms | {}ns\nConnect Time: {}ms | {}ns\nApp Connect Time: {}ms | {}ns\nPretransfer Time: {}ms | {}ns\nRedirect Time: {}ms | {}ns\nStartTransfer Time: {}ms | {}ns\nDownload Time: {}ms | {}ns\nTotal Time: {}ms | {}ns\nDownloaded: {} bytes\nSpeed: {} Mbps",
            self.headers,
            self.namelookup_time.as_millis(),
            self.namelookup_time.as_nanos(),
            self.connect_time.as_millis(),
            self.connect_time.as_nanos(),
            self.appconnect_time.as_millis(),
            self.appconnect_time.as_nanos(),
            self.pretransfer_time.as_millis(),
            self.pretransfer_time.as_nanos(),
            self.redirect_time.as_millis(),
            self.redirect_time.as_nanos(),
            self.starttransfer_time.as_millis(),
            self.starttransfer_time.as_nanos(),
            self.download_time.as_millis(),
            self.download_time.as_nanos(),
            self.total_time.as_millis(),
            self.total_time.as_nanos(),
            self.bytes.len(),
            self.speed
                )
    } 
}

fn make_request(url: &str) -> Result<CurlResult, anyhow::Error> {
    let mut handle = Easy::new();

    let mut buffer: Vec<u8> = vec![];

    let mut headers: Vec<u8> = vec![];

    handle.url(url)?;

    {
        let mut transfer = handle.transfer();

        transfer.write_function(|data| {
            buffer.extend_from_slice(data);
            Ok(data.len())
        })?;

        transfer.header_function(|header_data| {
            headers.extend_from_slice(header_data);
            true
        })?;

        transfer.perform()?;
    }

    let mut result = CurlResult{
        headers: String::from_utf8_lossy(&headers).to_string(),
        namelookup_time: handle.namelookup_time()?,
        connect_time: handle.connect_time()?,
        appconnect_time: handle.appconnect_time()?,
        pretransfer_time: handle.pretransfer_time()?,
        redirect_time: handle.redirect_time()?,
        starttransfer_time: handle.starttransfer_time()?,
        total_time: handle.total_time()?,
        download_time: handle.total_time()?,
        speed: 0,
        bytes: buffer
    };

    result.normalize();

    result.speed = ((result.bytes.len() as f32 * 0.000008) / result.download_time.as_secs_f32()) as usize; 

    Ok(result)

}
