

We're building a small world that's part of a much larger universe. Divide the work into tranches. Tranches are made of phases and phases are made of rounds. A tranche has an intent. phases, have intent. rounds may have inetint. phases have targets, goals. phases have targets. Rounds have goals, not acceptance criteria. Tranches and phases have targets targets are measurable, quantifiable numbers, real objective facts that may or may not be achieved.

Goals are open to interpretation, given the intente statement. Goals do require interpretation, and they should be clear. Goals once they've been set should be clear, but planning phases and other. Iterative agents may have to reinterpret goals that may involve rewording them. But once an implementation phase has started on a goal, it should be frozen for that goal for that agent.

Only when the big universe is sufficiently full, Will the small world to be believable.

The only way to go fast, is to go well.

Do one thing at a time.

Trust and verify.

Refactor and adversarial passes. Get a scope and a focus. So for example, the the scope may be the current diff, and the focus may be the current change. or the scope could be the entire tranche, and the focus may be general code, more performance or the test suite, security or gaps.

Planners, that is tranche, phase, or round planners plan different prompts in different agents as required for each of the thing by planning. It's a dynamic system not fixed in so stone cycles.

They include the scope and the focus. In their prompts. 

Importantly, planners are considering reports from previous rounds and folding that feedback back into the system moving forward. The system is designed to adapt to unknown unknowns.

I would assume that there would be a refactor or adversarial pass. In every round, and if nothing risky had changed in that round, that would free up the planner to allocate the refactor to a larger scope with a broader focus. If something risky has changed significant feature work, then the refactor would be targeted at that feature work.

The point is to try and strike a balance between moving forward towards finishing the phase and cycling back and patching up gaps. As they are identified and also adapting the cycling process to the project at hand.

The kinds of focuses for an adversarial pass are not identifiable before starting the project. only when we know the project do we know The Domain. And we know how to target critical areas that may have gaps. Planners fold in adversarial focus into Refactor phases when useful.

I would expect each round to have a refactor pass on the scope of the round and each phase to have a refactoring pass at the scope of the phase et. Cetera.

I do want to keep the refactor step in, but it needs to have a wider scope than just the current diff. It needs to be refactoring the current code base, given the new changes and how to refactor those new changes back into the larger system.

Against your best reasoning, I'm going to keep the TDD 3 cycles red green refactor.

Each step should return a report which contains what they did. the successes. What was difficult, where the time was spent? What they had to derive general comments and any gaps or anything worth flagging. In the compromises, they had to make



I want Red to also build the skeleton code for green. With a failing exception, throwing etc

I want a collection of phases to be called a milestone In this document I have referred to that that level as a phase but that would be confusing because I want to keep phase to refer to the individual step of TDD

each round gets its own log So each phase of each round's report gets appended to each rounds log

the lever that this flow relies on is an agent's ability to reason and choose the next best step

If it's possible for Red to create a skeleton is limited to shape only red should not be implementation details. Keep the isolation.

again we will keep the idea that a 1 test is one scenario these tests may be more like user stories maybe more like BDD style test. There could, and there even possibly should be, unit tests alongside. These are helpful but also may be discarded. Discarded if the code has changed significantly and the unit test needs to change or is no longer being helpful.

there is no halting clause. the idea is that the cycle continues until it's complete basically. So what we do have is exit ramps If a phase identifies something is unattainable, That just means we go back to planning step. We can come back to it later or desired a new approach we could break the acceptance criteria up into multiple phases of groundwork first


When each phase starts it starts a 30 minute timer.  at the 30 minute timer then the phase as a decision point. If it believes it can finish in the next 30 minutes it can go into overtime for a total of one hour. If there's not a clear path to success, then report back the findings. the finding is that was simply could not be achieved in 30 minutes and that's an excellent report that can go back to planning to try and find a way to restructure this goal. More groundwork, smaller change of tech. at the end of 60 minutes. Don't start any new endeavours report back


Red:
Useful interpetation from the first tes of the cycle red: "small enough to land in one round, meaningful enough to matter"
Catch hidden ordering problems the pre-baked grouping didn't anticipate.
read may be given a pre baked set of acceptance criteria mapping to around red now has an opportunity to either go with that or choose a different structure.
red should also maintain the test suite so it's performant rent should tag or categorise tests in a way that's easy for other phases to run specific collections of tests for example run all the tests in that round with a simple command. provide fast paths or anything else that may help with this test suite Be great to work with.
Confirm the round's pre-assigned AC block still makes sense given what's actually been built so far


all decisions can be decided without human input

If a phase needs to and it has uncommitted work it should commit that and clearly state what's going on.
Phases should leave the working tree clean

pull out common advice from all phases EG the exit clause into a cycle skill


I want timing reports of each phase round milestone tranche

so against all of your best advice I am going to pivot to Rust and I am going to start again.

the tranches will be physics, chemistry, biology and finally the glass pane.

physics will explore what we've done already but should include pressure Should include the elements that will be required Should it include temperature I want to a primitive kind of scenario for each element OK multiple ear scenarios would be kind of boring but for example a central column of sand falls to a pile. The central column of something else doesn't stay still. . A pool of water stays still. A column of water falls A level. rises cool air falls. that kind of stuff we've already seen it before. But as I said I want pressure and I want fluid dynamics so I'd like to see A U shaped pipe with air all around, Fill one half of the pipe with water The water should come to a level

scenarios should have their own empirical test i'm not sure if that existed already They should be able to pass or fail without human having to. So some of those primitive scenarios may not be run as the general full suite Then just be for regression or for completeness.

I like completeness

Anything else suitable for the physics tranche should go here



Chemistry
Now we will introduce the state changes. We can introduce burning wood.  I would like to explore iron and carbon and how to make steel in different ways to make rust.

another note I think I want dissolved guesses and gases in solids, as well as moisture in solids or moisture everywhere so about water and soil as well as air and soil and umm air dissolved in rubber and escaping a balloon. I'm not sure how much of this fits under physics.

explosions Particles should fly fly naturally and land naturally.

I think the full water cycle fits at this Tranche

Biology
I want fungus. And I want hummus. I want soil. I want bacteria I want slime mould. anything else like that.

here we see the carbon cycle and the nitrogen cycle



The glass pane is the pane through which the player sees the small world we have built. enough of the metaphors, we will obviously need a bunch of game/web interaction stuff going on early on so that I can see these scenarios play out. This is where we want to flesh out our interactions with a sandbox world or the terrarium world. However the terrarium world obviously doesn't have much use for coal and explosions and boiling but the sandbox world that I'm imagining does need all that. this is where I want to be able having it fully built Human interaction. I think I've explained what I want. Placing entities like the hot entity and the cold entity placing light sources. 


here's another thought
Would it help to have contiguous regions of gases all as fully mixed gas tracking its different components but it's or just the same stuff and we're just tracking as a separate entity the components of that stuff. Would this mixing work for water if we die in the water I don't think it would but I don't think I won't die in the water So if we had a dissolved gas in the water it would be assumed to be fully mixed throughout the water What does this approximation help the solver at all.


I'm a bit iffy about the deterministic requirement if we do genuinely want to move to the GPU and I think we will I'm really worried that these agents are going Trying to get determinism when we don't actually need it. "architecture contingent"


Refactor:
here I'm going to kind of overload Refactor a little bit I want it to be the big dog And it's actually going to be the Quality Gate as well It's the reviewer It's also making changes So it does have a big scope. but does it really it's not adding any features. so after Green and Red have done their thing refactor is going to cleaned it. It's fold  the new stuff into the existing code base. it's going to do Adversity tests cheques. And it's going to compile a report and ultimately give the signal about if we can move on to the next round or cycle on this same round with whatever updates are required.
Answering the question has the round goals been met to a sufficient standard. I will start with default model and effort level to be Sonnet and Medium; refactor can get Sonnet and High.

Refactor's job is difficult but it basically has 30 minutes to make the code base better. And it's up to that agent's best judgement how to do that. It's free time to improve the code base.




I don't like the use of 'acceptance criteria' I feel it's too clinical and associated with fixed and frozen or locked. I am relying on agents best judgement and report their decisions. Agents decisions should be trusted and verifed.





