use std::sync::{Arc};
use std::sync::Mutex;
use gtk4::prelude::IsA;
use gtk4::Widget;
use tracing::{debug, error};
use crate::components::{Children, Component};
use crate::components::detail_list_item::widget::{DetailListItemState, DetailListItemWidget};
use crate::state::{Dispatcher, Event, EventHandler, State};

pub struct DetailListItemComponent {
    pub widget: DetailListItemWidget,
    pub children: Vec<Arc<Mutex<Box<dyn EventHandler>>>>,
    state: Arc<Mutex<State>>,
    library_entry_id: Option<i32>
}

impl EventHandler for DetailListItemComponent {
    fn on_event(&mut self, _event: &Event) {

    }

    fn get_children(&self) -> Vec<Arc<Mutex<Box<dyn EventHandler>>>> {
        self.children.clone()
    }
}

impl Component<Option<()>> for DetailListItemComponent {
    fn new(state: Arc<Mutex<State>>, dispatcher: Arc<Mutex<Dispatcher>>, params: Option<()>) -> Self {
        let (widget, children) = Self::render(state.clone(), dispatcher.clone(), params);
        Self { widget, children, state, library_entry_id: None }
    }

    fn render(_state: Arc<Mutex<State>>, dispatcher: Arc<Mutex<Dispatcher>>, _params: Option<()>) -> (DetailListItemWidget, Children) {
        let widget = DetailListItemWidget::new(dispatcher.clone());

        (widget, vec![])
    }

    fn update(&mut self) {
        debug!("update detail_list_item");
        let playing_library_entry_id = self.state.lock().expect("could not lock state").playing_library_entry.clone().map(|entry| entry.id);
        let entry_with_position = self.state.lock().expect("could not lock state")
            .library_entry
            .children
            .clone()
            .and_then(|children| {
                let position = children.iter().position(|child| child.id == self.library_entry_id.unwrap());
                position.and_then(move |pos| children.get(pos).cloned().map(|entry| (pos, entry)))
            });

        match entry_with_position {
            Some((position, entry)) => {
                self.widget.set_library_entry(entry.clone());
                self.widget.set_position(position as u32);
                self.widget.set_name(&entry.name);
                // Is this currently playing?
                if entry.id == playing_library_entry_id.unwrap_or(-1) {
                    self.widget.set_state(DetailListItemState::Playing);
                }
                else if let Some(_) = entry.played_at.as_ref() {
                    self.widget.set_state(DetailListItemState::Played);
                }
                else {
                    self.widget.set_state(DetailListItemState::None);
                }
            },
            None => {
                error!("Wanted to render detail list item but not having children? o.O");
            }
        }
        debug!("update detail_list_item done");
    }

    fn get_widget(&self) -> impl IsA<Widget> {
        self.widget.clone()
    }
}
