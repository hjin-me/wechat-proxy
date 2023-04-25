use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[allow(non_snake_case)]
#[component]
pub fn App(cx: Scope) -> impl IntoView {
    provide_meta_context(cx);
    let formatter = |text| format!("{text}");

    view! {
        cx,
        <Html lang="zh-hans"/>
        <Title
      // reactively sets document.title when `name` changes
      text="wechat proxy"
      // applies the `formatter` function to the `text` value
      formatter=formatter
    />
    <div>
        <h1>"Wechat Proxy"</h1>
    </div>
    }
}
