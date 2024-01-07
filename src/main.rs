#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]
use cyw43_pio::PioSpi;
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::{yield_now};
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_sync::{
    channel::Channel,
    blocking_mutex::raw::CriticalSectionRawMutex,
};
use embassy_rp::peripherals::{
    DMA_CH0,
    PIN_15,
    PIN_16,
    PIN_17,
    PIN_18,
    PIN_19,
    PIN_20,
    PIN_21,
    PIN_22,
    PIN_23,
    PIN_25,
    PIN_26,
    PIN_27,
    PIN_28,
    PIO0
};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

mod virtual_keyboard_matrix;
use virtual_keyboard_matrix::*;

mod controller;
use controller::*;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

static CONTROLLER_CHANNEL: Channel<CriticalSectionRawMutex, ControlMessages, 10> = Channel::new();

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<'static, Output<'static, PIN_23>, PioSpi<'static, PIN_25, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let p = embassy_rp::init(Default::default());

    let fw = include_bytes!("../../embassy/cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../../embassy/cyw43-firmware/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download 43439A0.bin --format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download 43439A0_clm.bin --format bin --chip RP2040 --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);

    let a = Output::new(p.PIN_15, Level::Low);
    let b = Output::new(p.PIN_16, Level::Low);
    let c = Output::new(p.PIN_17, Level::Low);
    let d = Output::new(p.PIN_18, Level::Low);
    let e = Output::new(p.PIN_19, Level::Low);
    let f = Output::new(p.PIN_20, Level::Low);

    let up = Output::new(p.PIN_21, Level::Low);
    let down = Output::new(p.PIN_22, Level::Low);
    let left = Output::new(p.PIN_26, Level::Low);
    let right = Output::new(p.PIN_27, Level::Low);

    let start = Output::new(p.PIN_28, Level::Low);
    
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(&mut pio.common, pio.sm0, pio.irq0, cs, p.PIN_24, p.PIN_29, p.DMA_CH0);

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    
    if let Err(why) = spawner.spawn(wifi_task(runner)) {
	defmt::panic!("Failed starting the WIFI hardware task. {}",why);
    }

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::Performance)
        .await;

    // Use a link-local address for communication without DHCP server
    let config = Config::dhcpv4(Default::default());

    // Generate random seed
    let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guarenteed to be random.

    // Init network stack
    static STACK: StaticCell<Stack<cyw43::NetDriver<'static>>> = StaticCell::new();
    static RESOURCES: StaticCell<StackResources<2>> = StaticCell::new();
    let stack = &*STACK.init(Stack::new(
        net_device,
        config,
        RESOURCES.init(StackResources::<2>::new()),
        seed,
    ));

    if let Err(why) = spawner.spawn(net_task(stack)){
	defmt::panic!("Failed starting the NET stack: {}",why);
    }

    if let Err(why) = spawner.spawn(socket_handler(
	&stack,
	control	    
    )){
	defmt::panic!("Failed starting web server task: {}",why);
    }


    if let Err(why) = spawner.spawn(pin_handler(
	a,
	b,
	c,
	d,
	e,
	f,
	up,
	down,
	left,
	right,
	start,
	    
    )){
	defmt::panic!("Failed starting web server task: {}",why);
    }

    
}

#[embassy_executor::task]
async fn pin_handler(
    mut a: Output<'static, PIN_15>,
    mut b: Output<'static, PIN_16>,
    mut c: Output<'static, PIN_17>,
    mut d: Output<'static, PIN_18>,
    mut e: Output<'static, PIN_19>,
    mut f: Output<'static, PIN_20>,
    mut up: Output<'static, PIN_21>,
    mut down: Output<'static, PIN_22>,
    mut left: Output<'static, PIN_26>,
    mut right: Output<'static, PIN_27>,
    mut start: Output<'static, PIN_28>,
){

    macro_rules! push_and_release{
	($pin: expr) =>{
	    $pin.set_high();
	    Timer::after(Duration::from_millis(50)).await;
	    $pin.set_low();
	    Timer::after(Duration::from_millis(50)).await;
	}
    }
    
    let receiver = CONTROLLER_CHANNEL.receiver();
    
    loop{

	info!("waiting incomming ControllMessage..");
	let message = receiver.receive().await;

	match message {
	    ControlMessages::A => {
		push_and_release!(a);
	    },
	    ControlMessages::B => {
		push_and_release!(b);
	    },
	    ControlMessages::C => {
		push_and_release!(c);
	    },
	    ControlMessages::D => {
		push_and_release!(d);
	    },
	    ControlMessages::E => {
		push_and_release!(e);
	    },
	    ControlMessages::F => {
		push_and_release!(f);
	    },
	    ControlMessages::Up => {
		push_and_release!(up);
	    },
	    ControlMessages::Down => {
		push_and_release!(down);
	    },
	    ControlMessages::Left => {
		push_and_release!(left);
	    },
	    ControlMessages::Right => {
		push_and_release!(right);
	    },
	    ControlMessages::Start => {
		push_and_release!(start);
	    },
	}

	yield_now().await;
	
    }

}

#[embassy_executor::task]
async fn socket_handler(
    stack: &'static Stack<cyw43::NetDriver<'static>>,
    mut control: cyw43::Control<'static>,	
){

    const WIFI_NETWORK : &str = "COPOLAND-PLUS";
    const WIFI_PASSWORD : &str = "Nhy6bgt5vfr4.";

    loop {
        //control.join_open(WIFI_NETWORK).await;
        match control.join_wpa2(WIFI_NETWORK, WIFI_PASSWORD).await {
            Ok(_) => break,
            Err(err) => {
                info!("failed to join WIFI with status={}", err.status);
            }
        }
    }

    // Wait for DHCP, not necessary when using static IP
    info!("waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    info!("DHCP is now up!");

    

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];
    let controller_sender = CONTROLLER_CHANNEL.sender();
    const KEYBOARD_MODE : &u8 = &b'0';
    const DIRECT_MODE : &u8 = &b'1';



    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
	socket.set_timeout(Some(Duration::from_secs(10)));

        control.gpio_set(0, false).await;
        info!("Listening on port TCP 1234...");
        if let Err(e) = socket.accept(1234).await {
            warn!("error tryinto to accept client: {:?}", e);
            continue;
        }

        info!("Received connection from client {:?}", socket.remote_endpoint());
        control.gpio_set(0, true).await;

	let mut location : VirtualKeyaboardMatrixItem = VirtualKeyaboardMatrixItem::N1;

	macro_rules! answer_ok{
	    () => {
		match socket.write_all(b"OK\n").await {
		    Ok(()) => {
			let _ = socket.flush();
			yield_now().await;
			let _ = socket.close();
			yield_now().await;
			break;
		    }
		    Err(e) => {
			warn!("write response to client error: {:?}", e);
			break;
		    }
		};
	    }
	}

	macro_rules! send_to_controller {
	    ($message : expr) => {
		
		controller_sender.send($message).await;
		yield_now().await;
		
	    }
	}

        'socket_loop : loop {
	    info!("reading socket...");
            match socket.read(&mut buf).await {
                Ok(0) => {
		    info!("No incomming bytes...");
		    yield_now().await;
                }
                Ok(n) => {

		    info!("Incomming bytes {}",n);

		    if let Some(KEYBOARD_MODE) = buf.get(0) {
		    
			for item in VirtualKeyaboardMatrixItem::convert_bytes(&buf[1..n]) {

			    if let VirtualKeyaboardMatrixItem::EOL = item {
				answer_ok!();
			    }
			    
			    if let Some((delta_x, delta_y)) = location.compute_move_delta(&item) {

				if delta_x > 0 {
				    for _ in 0..delta_x {
					send_to_controller!(ControlMessages::Right);
				    }
				}

				if delta_x < 0 {
				    for _ in 0..delta_x {
					send_to_controller!(ControlMessages::Left);
				    }
				}

				if delta_y < 0 {
				    for _ in 0..delta_y {
					send_to_controller!(ControlMessages::Up);
				    }
				}


				if delta_y < 0 {
				    for _ in 0..delta_y {
					send_to_controller!(ControlMessages::Down);
				    }
				}

				send_to_controller!(ControlMessages::A);
				
				location = item;
			    }
			    
			}

			
		    } else if let Some(DIRECT_MODE) = buf.get(0) {

			for potential_message in buf.iter().take(n).map(|x| ControlMessages::from_byte(*x) ) {
			    if let Some(message) = potential_message{
				controller_sender.send(message).await;
			    }
			}
			
			answer_ok!();
		    }

		    
		},
                Err(e) => {
                    warn!("socket read error: {:?}", e);
                    break 'socket_loop;
                }
            };

        }
    }
    
}
