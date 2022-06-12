use synesthesia::Synesthesia;

fn main()  {
    let mut synesthesia: Synesthesia = Synesthesia::init();
    synesthesia.load_scene(&std::env::args().nth(1).expect("please provide a sound sample"));
    synesthesia.run()
}