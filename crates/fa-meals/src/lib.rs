//! fa-meals: recipes, meal plan, templates and the living grocery list (M5, arch/Meals.md).

pub mod catalogue;
mod change;
mod contrib;
pub mod defaults;
pub mod error;
pub mod grocery;
pub mod image;
pub mod jobs;
pub mod matching;
pub mod normalise;
pub mod plan;
pub mod recipes;
pub mod seed_data;
pub mod service;
pub mod settings;
pub mod templates;

pub use defaults::ensure_defaults;
pub use error::{MealsError, Result};
pub use jobs::{ImageOutcome, IngredientLlmResult, LlmOutcome, default_schedules, handlers};
pub use recipes::{RecipeImageData, RecipeListQuery};
pub use service::{ActorRef, Clock, MealsService, UNDO_MINUTES};
