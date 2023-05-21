# Hayt2

> A DEEPLY ill-advized fork of [the original, Name-Brand Hayt][hayt].

[hayt]: https://github.com/desert-planet/hayt

## Setup Instructions

This should match the [Serenity setup instructions][serenity-setup], but
basically:

1. Make sure you have Discord running locally.
    * If you're living in WSL land like me, you probs want `apt install discord`
    and then you can run `discord` in a terminal to get some shit idk.
    * TODO: Move that shit to some kind of callout??
1. Clone the Serenity repo into your workspace.
    * git clone https://github.com/serenity-rs/serenity.git
    * TODO: Submodule? Subtree? Save people a step, if possible.
1. Uhhhhhh run it I guess? `cargo make 1` will run a ping/pong example as a
   start.
    * TODO: We MIGHT be missing a step here for installing `cargo make`, but idk
      we have `make` at home.

[serenity-setup]: https://github.com/serenity-rs/serenity/blob/94ed67bfeb1821e3f212a9057c50c4e3b95916f2/examples/README.md
