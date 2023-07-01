// Antiquated ASL version, activate only in case of the downfall of The New Runtime.
state("null_beta2")
{
    bool isMainMenu: 0x43C924;
    bool isFinished: 0x43C925;
    byte roomNumber: 0x43C926;
    byte hours: 0x43C927;
    byte minutes: 0x43C928;
    byte seconds: 0x43C929;
    byte frame: 0x43C92A;
}

startup
{
    // vars.Log = (Action<object>)((output) => print("[Process ASL] " + output));
    settings.Add("room", false, "Split after each room");
    settings.Add("chapter", true, "Split after each chapter");
    vars.SplitsDone = new HashSet<int>();
}

start
{
    return !current.isMainMenu && old.isMainMenu;
}

reset
{
    return current.isMainMenu && !old.isMainMenu;
}

onReset
{
    vars.SplitsDone.Clear();
}

gameTime
{
    double seconds = current.hours * 3600 + 
        current.minutes * 60 + 
        current.seconds + 
        (current.frame / 60.0);
    return TimeSpan.FromSeconds(seconds);
}

split
{
    if (current.isFinished && !old.isFinished) {
        return true;
    }

    if (current.roomNumber > old.roomNumber) {
        bool isChapter = current.roomNumber % 4 == 0;
        return (settings["room"] || (settings["chapter"] && isChapter)) &&
            vars.SplitsDone.Add(old.roomNumber);
    }
}

isLoading
{
    return true; // Stops timer flickering
}