//! This example uses the RP Pico W board Wifi chip (cyw43).
//! Creates an Access point Wifi network and creates a TCP endpoint on port 1234.

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
    PIN_0,
    PIN_1,
    PIN_2,
    PIN_3,
    PIN_4,
    PIN_5,
    PIN_6,
    PIN_7,
    PIN_8,
    PIN_9,
    PIN_10,
    PIN_23,
    PIN_25,
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

    let a = Output::new(p.PIN_0, Level::Low);
    let b = Output::new(p.PIN_1, Level::Low);
    let c = Output::new(p.PIN_2, Level::Low);
    let d = Output::new(p.PIN_3, Level::Low);
    let e = Output::new(p.PIN_4, Level::Low);
    let f = Output::new(p.PIN_5, Level::Low);

    let up = Output::new(p.PIN_6, Level::Low);
    let down = Output::new(p.PIN_7, Level::Low);
    let left = Output::new(p.PIN_8, Level::Low);
    let right = Output::new(p.PIN_9, Level::Low);

    let start = Output::new(p.PIN_10, Level::Low);
    
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
    let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::new(169, 254, 1, 1), 16),
        dns_servers: heapless::Vec::new(),
        gateway: None,
    });

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
    mut a: Output<'static, PIN_0>,
    mut b: Output<'static, PIN_1>,
    mut c: Output<'static, PIN_2>,
    mut d: Output<'static, PIN_3>,
    mut e: Output<'static, PIN_4>,
    mut f: Output<'static, PIN_5>,
    mut up: Output<'static, PIN_6>,
    mut down: Output<'static, PIN_7>,
    mut left: Output<'static, PIN_8>,
    mut right: Output<'static, PIN_9>,
    mut start: Output<'static, PIN_10>,
){

    macro_rules! push_and_release{
	($pin: expr) =>{
	    $pin.set_high();
	    Timer::after(Duration::from_millis(200)).await;
	    $pin.set_low();
	}
    }
    
    let receiver = CONTROLLER_CHANNEL.receiver();
    
    loop{

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
	
    }

}

#[embassy_executor::task]
async fn socket_handler(
    stack: &'static Stack<cyw43::NetDriver<'static>>,
    mut control: cyw43::Control<'static>,	
){

    //control.start_ap_open("cyw43", 5).await;
    control.start_ap_wpa2("pandora13-wifi-controller", "Nhy6bgt5vfr4.", 5).await;
    
    // And now we can use it!

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];
    let controller_sender = CONTROLLER_CHANNEL.sender();
    const KEYBOARD_MODE : &u8 = &0;
    const DIRECT_MODE : &u8 = &1;


    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        control.gpio_set(0, false).await;
        info!("Listening on TCP:1234...");
        if let Err(e) = socket.accept(1234).await {
            warn!("accept error: {:?}", e);
            continue;
        }

        info!("Received connection from {:?}", socket.remote_endpoint());
        control.gpio_set(0, true).await;

	let mut location : VirtualKeyaboardMatrixItem = VirtualKeyaboardMatrixItem::N1;

	macro_rules! answer_ok{
	    () => {
		match socket.write_all(b"OK\n").await {
		    Ok(()) => {}
		    Err(e) => {
			warn!("write response error: {:?}", e);
			break;
		    }
		};

		yield_now().await;
	    }
	}

	macro_rules! send_to_controller {
	    ($message : expr) => {
		
		controller_sender.send($message).await;
		//yield_now().await;
		
	    }
	}

        'socket_loop : loop {
            match socket.read(&mut buf).await {
                Ok(0) => {
		    yield_now().await;
                }
                Ok(n) => {

		    if let Some(KEYBOARD_MODE) = buf.get(0) {
		    
			for item in VirtualKeyaboardMatrixItem::convert_bytes(&buf[1..n]) {

			    if let VirtualKeyaboardMatrixItem::EOL = item {
				answer_ok!();
				break;
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

			buf.iter_mut().for_each(|x|{ *x = 0; }); 
			
		    } else if let Some(DIRECT_MODE) = buf.get(0) {

			if let Some(message) = buf.get(1).and_then(|x| ControlMessages::from_byte(*x) ) {
			    controller_sender.send(message).await;

			    answer_ok!();
			}
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
