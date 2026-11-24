# Check what a probability can tell you

Try **File > Load Template > Entanglement > quantum teleportation**. The sender's message starts on q0, and q2 is the receiver. Click **Prepare message** (step 4) and note the 25% chance of reading 1 on q0.

Now inspect q2 at steps **6**, **7** and **8**. Which correction restores that probability? What does the other correction change?

**Check your prediction.** At step 6, q2 reads 1 half the time. The X correction at step 7 restores the 25% probability. Follow the joint row `101` from step 7 to step 8. Its amplitude changes from `−0.25i` to `+0.25i`. The Z correction restores the message's relative phase.

Choose **Template guide** for more circuits to explore, or **Error correction** to compare single and double faults that produce the same two check bits.
