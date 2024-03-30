use wasm_bindgen::prelude::*;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};
extern crate console_error_panic_hook;

const PROTOCOL_VERSION: &'static str = include_str!("../../protocol_version");

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
#[derive(Clone)]
struct AlarmClient {
    ws: WebSocket,
}

#[wasm_bindgen]
impl AlarmClient {
    #[wasm_bindgen(constructor)]
    pub fn new(addr: &str) -> Result<AlarmClient, JsValue> {
        console_error_panic_hook::set_once();

        let ws = WebSocket::new(addr)?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let on_open_cb = Self::on_open_default(&ws);
        ws.set_onopen(Some(on_open_cb.as_ref().unchecked_ref()));
        on_open_cb.forget();

        let on_message_cb = Self::on_message_default();
        ws.set_onmessage(Some(on_message_cb.as_ref().unchecked_ref()));
        on_message_cb.forget();

        let on_error_cb = Self::on_error_default();
        ws.set_onerror(Some(on_error_cb.as_ref().unchecked_ref()));
        on_error_cb.forget();

        Ok(Self { ws })
    }

    pub fn acknowledge(&self, id: &str) -> Result<(), JsError> {
        match self.ws.send_with_str(&std::format!("::ack::{id}")) {
            Ok(_) => console_log!("message successfully sent"),
            Err(err) => console_log!("error sending message: {:?}", err),
        }

        Ok(())
    }

    pub fn send_keep_alive(&self) -> Result<(), JsError> {
        match self.ws.send_with_str("::ka::") {
            Ok(_) => console_log!("message successfully sent"),
            Err(err) => console_log!("error sending message: {:?}", err),
        }

        Ok(())
    }

    pub fn subscribe(&self, alm: &str) {
        self.ws.send_with_str(&std::format!("::subscribe::{alm}"));
    }

    pub fn close(&self) {
        self.ws.close();
    }

    pub fn set_onopen(&mut self, cb: js_sys::Function) {
        let cloned_ws = self.ws.clone();
        let onopen_callback = Closure::<dyn FnMut()>::new(move || {
            console_log!("socket opened");
            let _ = cloned_ws.send_with_str(PROTOCOL_VERSION);

            let this = JsValue::null();
            let _ = cb.call0(&this);
        });

        self.ws
            .set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();
    }

    pub fn set_onmessage(&mut self, cb: js_sys::Function) {
        let this = JsValue::null();
        let self_cloned = self.clone();

        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            // Handle difference Text/Binary,...
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                console_log!("message event, received Text: {:?}", txt);
                if txt.starts_with("::protocol_version::", 0) {
                    console_log!("got server version!");
                } else {
                    let _ = cb.call1(&this, &txt);
                    let _ = self_cloned.ws.is_array();
                }
            } else {
                console_log!("message event, received Unknown: {:?}", e.data());
                let _ = cb.call1(&this, &e.data());
            }
        });

        self.ws
            .set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();
    }

    pub fn set_onerror(&mut self, cb: js_sys::Function) {
        let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
            console_log!("error event: {:?}", e);
            let this = JsValue::null();
            let _ = cb.call1(&this, &e);
        });

        self.ws
            .set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();
    }

    fn on_open_default(ws: &WebSocket) -> Closure<dyn FnMut()> {
        let cloned_ws = ws.clone();
        Closure::<dyn FnMut()>::new(move || {
            console_log!("on open");
            let _ = cloned_ws.send_with_str(PROTOCOL_VERSION);
        })
    }

    fn on_message_default() -> Closure<dyn FnMut(MessageEvent)> {
        Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                console_log!("message event, received Text: {:?}", txt);
            } else {
                console_log!("message event, received Unknown: {:?}", e.data());
            }
        })
    }

    fn on_error_default() -> Closure<dyn FnMut(ErrorEvent)> {
        Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
            console_log!("error event: {:?}", e);
        })
    }
}

#[wasm_bindgen]
pub fn hello(msg: &str) -> String {
    std::format!("Hello World: {msg}")
}
