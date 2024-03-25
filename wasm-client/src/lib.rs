use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
struct AlarmClient {
    ws: WebSocket,
    pub onOpen: &'static js_sys::Function,
    pub onMessage: &'static js_sys::Function,
    pub onError: &'static js_sys::Function,
}

#[wasm_bindgen]
impl AlarmClient {
    #[wasm_bindgen]
    pub fn connect(&mut self, addr: &str, port: i32, ssl: bool) -> Result<(), JsValue> {
        let protocol = if ssl { "wss" } else { "ws" };
        self.ws = WebSocket::new(&std::format!("{protocol}://{addr}:{port}"))?;
        self.ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        self.set_onopen_cb();

        Ok(())
    }

    fn set_onopen_cb(self: Rc<Self>) {
        let cloned_ws = self.ws.clone();

        let onopen_callback = Closure::<dyn FnMut()>::new(move || {
            console_log!("socket opened");

            let this = JsValue::null();
            let _ = self.onOpen.call0(&this);

            match cloned_ws.send_with_str("ping") {
                Ok(_) => console_log!("message successfully sent"),
                Err(err) => console_log!("error sending message: {:?}", err),
            }
            // send off binary message
            match cloned_ws.send_with_u8_array(&[0, 1, 2, 3]) {
                Ok(_) => console_log!("binary message successfully sent"),
                Err(err) => console_log!("error sending message: {:?}", err),
            }
        });
        self.ws
            .set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();
    }

    fn set_onmessage_cb(self: Rc<Self>) {
        let cloned_ws = self.ws.clone();

        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            // Handle difference Text/Binary,...
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                console_log!("message event, received arraybuffer: {:?}", abuf);
                let array = js_sys::Uint8Array::new(&abuf);
                let len = array.byte_length() as usize;
                console_log!("Arraybuffer received {}bytes: {:?}", len, array.to_vec());
                // here you can for example use Serde Deserialize decode the message
                // for demo purposes we switch back to Blob-type and send off another binary message
                cloned_ws.set_binary_type(web_sys::BinaryType::Blob);
                match cloned_ws.send_with_u8_array(&[5, 6, 7, 8]) {
                    Ok(_) => console_log!("binary message successfully sent"),
                    Err(err) => console_log!("error sending message: {:?}", err),
                }
            } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
                console_log!("message event, received blob: {:?}", blob);
                // better alternative to juggling with FileReader is to use https://crates.io/crates/gloo-file
                let fr = web_sys::FileReader::new().unwrap();
                let fr_c = fr.clone();
                // create onLoadEnd callback
                let onloadend_cb =
                    Closure::<dyn FnMut(_)>::new(move |_e: web_sys::ProgressEvent| {
                        let array = js_sys::Uint8Array::new(&fr_c.result().unwrap());
                        let len = array.byte_length() as usize;
                        console_log!("Blob received {}bytes: {:?}", len, array.to_vec());
                        // here you can for example use the received image/png data
                    });
                fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
                fr.read_as_array_buffer(&blob).expect("blob not readable");
                onloadend_cb.forget();
            } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                console_log!("message event, received Text: {:?}", txt);
            } else {
                console_log!("message event, received Unknown: {:?}", e.data());
            }
        });
        // set message event handler on WebSocket
        self.ws
            .set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        // forget the callback to keep it alive
        onmessage_callback.forget();
    }

    fn set_onerror_cb(self: Rc<Self>) {
        let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
            console_log!("error event: {:?}", e);
            let this = JsValue::null();
            let _ = self.onError.call1(&this, &e);
        });
        self.ws
            .set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();
    }
}

#[wasm_bindgen]
pub fn hello(msg: &str) -> String {
    std::format!("Hello World: {msg}")
}
