use std::path::PathBuf;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(crate) enum RigCommand {
    /// Find deterministic opaque connected-component bounds in an indexed frame.
    Parts {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        revision: Option<String>,
        #[arg(long)]
        frame: Option<String>,
        #[arg(long, default_value_t = 1)]
        min_pixels: u32,
    },
    /// Create a one-pose rig without authoring JSON files.
    Assemble {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: String,
        #[arg(long)]
        source_frame: Option<String>,
        #[arg(long)]
        width: u32,
        #[arg(long)]
        height: u32,
        /// `ID,X,Y,WIDTH,HEIGHT,PIVOT_X,PIVOT_Y`. Repeat for every component.
        #[arg(long = "part", required = true)]
        parts: Vec<String>,
        /// `ID,PARENT_OR_NONE,PART_ID,X,Y,DEPTH`. Repeat in hierarchy order.
        #[arg(long = "node", required = true)]
        nodes: Vec<String>,
        #[arg(long, default_value = "pose-0001")]
        pose: String,
        #[arg(long, default_value = "Pose 1")]
        name: String,
        #[arg(long, default_value_t = 100)]
        duration: u32,
        #[arg(long, default_value_t = 0)]
        inbetweens: u16,
        #[arg(long)]
        looped: bool,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    /// Crop parts from an indexed revision and create its initial rig and poses.
    Create {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: String,
        #[arg(long)]
        source_frame: Option<String>,
        /// pixelate.rig-definition/v1 JSON document.
        #[arg(long)]
        definition: PathBuf,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    /// Apply one typed rig mutation JSON document.
    Mutate {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: String,
        /// A tagged `RigMutation` JSON object, such as `update_node`.
        #[arg(long)]
        mutation: PathBuf,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    /// Update one node in one pose using normal pixel/degree/percent units.
    Node {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: String,
        #[arg(long)]
        pose: String,
        #[arg(long)]
        node: String,
        #[arg(long, allow_hyphen_values = true)]
        x: Option<i32>,
        #[arg(long, allow_hyphen_values = true)]
        y: Option<i32>,
        #[arg(long, allow_hyphen_values = true)]
        rotation: Option<i32>,
        #[arg(long, allow_hyphen_values = true)]
        width_percent: Option<i32>,
        #[arg(long, allow_hyphen_values = true)]
        height_percent: Option<i32>,
        #[arg(long, allow_hyphen_values = true)]
        layer: Option<i32>,
        #[arg(long)]
        part: Option<String>,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    /// Swap two nodes' assigned sprites in every pose.
    Swap {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: String,
        #[arg(long)]
        first: String,
        #[arg(long)]
        second: String,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    /// Set automatic in-between count and optional loop closure.
    Interpolation {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: String,
        #[arg(long)]
        inbetweens: u16,
        #[arg(long)]
        looped: bool,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    /// Set the shared duration of all rendered rig frames.
    Duration {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: String,
        #[arg(long)]
        duration: u32,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    /// Duplicate a manual pose immediately after its source.
    DuplicatePose {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: String,
        #[arg(long)]
        pose: String,
        #[arg(long)]
        new_pose: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    /// Delete a manual pose (the final pose cannot be deleted).
    DeletePose {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: String,
        #[arg(long)]
        pose: String,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    /// Move a manual pose to a zero-based position.
    ReorderPose {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: String,
        #[arg(long)]
        pose: String,
        #[arg(long)]
        position: usize,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    /// Rename a manual pose.
    RenamePose {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: String,
        #[arg(long)]
        pose: String,
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    /// Remove rig authoring data while preserving every rendered frame.
    Bake {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: String,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
}
