# textpocalypse
A text-based post-apocalyptic survival game.

Currently in the pre-prototype phase. AKA it's not really any kind of "game" yet.

Goals:
* fun (duh)
* moddability
* support for screen readers
* gentle learning curve
* complex and punishing, but not to the point of being overly frustrating

Non-goals:
* fancy graphics
* hyper-realism
* having as many features as possible
* being an MMO

Planned features (in no particular order):
- [ ] base building
- [x] "survival needs" (hunger, thirst, etc.)
- [x] stats/skills
- [ ] combat
- [ ] crafting
- [x] mini-map
- [ ] audio
- [ ] environmental hazards (extreme cold, radiation, etc.)
- [ ] multiplayer
  - [x] allowing multiple players to connect to the same world
  - [ ] maintaining player state when you reconnect
- [ ] procedurally generated world
- [ ] saving and loading worlds
- [ ] player preferences

## How to play
1. Clone this repo
2. `cargo run`
3. Point your favorite MUD client or SSH client to `localhost` port 8080 (I recommend [MUSHclient](https://www.gammon.com.au/downloads/dlmushclient.htm))

### Recommended MUSHclient settings
* Appearance -> Output -> Font: Lucida Console, Regular, 10 pt
* Appearance -> Output -> Spacing -> Line spacing (pixels): 14
* If you don't have/like Lucida Console, [Liberation Mono](https://www.fontsquirrel.com/fonts/liberation-mono) is also pretty good

## Contributing
I'm currently not looking for external contributions to this project. If you want to suggest a minor bugfix, I might consider it though.
