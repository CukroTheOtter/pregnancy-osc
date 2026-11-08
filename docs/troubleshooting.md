# Troubleshooting

Start with what you're seeing. Short versions of these are in the app too, under **Settings > 🩹 Troubleshooting**. Its **Checkup** shows whether VRChat, OSC, your avatar and the logs are working, and finds problems in your save or settings (like **Only my partners** on with no partners, or a pregnancy whose WooHoo is gone), each with a button to fix it.

The app learns what's on an avatar from VRChat. If VRChat reports an old list of parameters, the app can only show that old list.

- [The app says an avatar doesn't have the Pregnancy System](#the-app-says-an-avatar-doesnt-have-the-pregnancy-system)
- [A feature is missing after you uploaded an avatar](#a-feature-is-missing-after-you-uploaded-an-avatar)
- [The belly isn't changing](#the-belly-isnt-changing)
- [It says "No OSC" or "Waiting for VRChat"](#it-says-no-osc-or-waiting-for-vrchat)
- [Labor won't start, or is paused](#labor-wont-start-or-is-paused)
- [WooHoos aren't being detected](#woohoos-arent-being-detected)
- [The other parent is unknown](#the-other-parent-is-unknown)
- [The weeks and the wait don't seem to match](#the-weeks-and-the-wait-dont-seem-to-match)
- [The chatbox status disappeared](#the-chatbox-status-disappeared)
- [Asking for help](#asking-for-help)

## The app says an avatar doesn't have the Pregnancy System

If you know it does, try these in order:

1. **Wear the avatar and wait about 30 seconds.** The warning can be from an old saved list. Wearing the avatar makes the app read it again.
2. **Reset the avatar's OSC config.** In VRChat: Action Menu > Options > OSC > **Reset Config**. Then switch to another avatar and back.
3. **Check the upload.** In Unity, make sure the Pregnancy System is on the avatar, then upload it again.

To see what the app has read, open **Avatars > Edit > What this avatar can do**. A ✓ means VRChat listed that parameter, and a – means it didn't.

The **Your avatars** list doesn't mean an avatar has the system. An avatar is also listed there if you're wearing it or have set something up on it, like a pregnancy.

## A feature is missing after you uploaded an avatar

After an upload, VRChat can keep reporting the avatar's old parameters. Use **Reset Config** as above, then switch avatars and back.

If it's still missing, check that the feature's parameter is on the uploaded avatar. They're all listed in [osc-parameters.md](osc-parameters.md).

## The belly isn't changing

- **Hide Belly is on.** Home shows a 🙈 note with a **Show** button.
- **You're in a public instance.** By default the belly is hidden in Public and Group Public. Tap **Show here** on Home, or change it in **Settings > Safety**.
- **The pregnancy is paused.** While paused, the app leaves the belly alone so you can set it in-game.
- **You're waiting to find out.** The belly stays flat until the reveal.
- **Your avatar has its own name for the belly.** Set it in **Avatars > Edit > What this avatar can do**.

## It says "No OSC" or "Waiting for VRChat"

- **"Waiting for VRChat":** the app doesn't see VRChat running yet, and links up by itself when it does.
- **"No OSC":** turn it on in VRChat: Action Menu > Options > OSC > **Enabled**.
- **Using an OSC router?** Set your ports in **Settings > VRChat**.

## Labor won't start, or is paused

Labor only starts or moves while you're in VRChat, on an avatar that has `Pregnancy/Labor`. Otherwise, check:

- **Timing:** **Surprise** picks a day from week 37 to 42 (earlier for twins and more), and **When I say** waits for you. See **Settings > Labor and birth**.
- **Instance type:** by default labor waits in Public and Group Public (**Settings > Safety**).
- **AFK:** labor stops while VRChat says you're AFK.
- **Resting holds off labor:** if this is on, labor only starts while you're standing.
- **Labor waits for the other parent:** if this is on, the one you're pregnant by has to be in the instance.

Home's **Welcome your baby** always skips straight to the birth.

## WooHoos aren't being detected

- **Use SPS sockets** has to be on (**Settings > Detecting WooHoos**). That section also lists the sockets on the avatar you're wearing and whether each one counts.
- **No sockets listed?** Sockets need VRCFury haptics turned on, on your avatar.
- **The hole type has to count.** A socket gets its type from its name. You can fix the type under **Avatars > Edit > SPS sockets**.
- **Already pregnant?** WooHoos are ignored during a pregnancy, except that one can start labor if you've turned that on.
- **Only my partners on?** Then a WooHoo only counts with someone under 💞 My partners (Partners tab). Others are kept as "Not a partner".

## The other parent is unknown

This is normal. VRChat's log says who's in the instance, but not who you WooHoo'd with. The app only fills in a name by itself when:

- There's exactly one other person, and nobody has joined or left for 10 minutes.
- You've WooHoo'd in that instance as many times as there are other people (2 or more). It then picks someone at random.

Otherwise you can pick who it was on the "You WooHoo'd" card. **Read VRChat logs** (**Settings > Logs and pictures**) has to be on for the app to see who's there.

## The weeks and the wait don't seem to match

Only the **weeks** are scaled. Week 20 of a 1-week pregnancy is about 3.5 real days in.

Hours, days and dates are always **real time**. That includes the time left on the rings, the due date, and the wait to find out.

The wait after a WooHoo is 1/20 of your pregnancy length unless you pick your own under **Settings > Fertility > Find out after**. For example, it's 8 h 24 min on a 1-week pregnancy.

## The chatbox status disappeared

By default the status is off while your belly is hidden, including when it's hidden by **Safety** in a public instance. Turn off **Hide the status while my belly is hidden** under **Settings > Chatbox** to always show it.

## Asking for help

Tap **Copy details** in **Settings > 🩹 Troubleshooting** and paste it with your question. You'll see the text before you copy it. It has the name and ID of the avatar you're wearing, but nothing about other people, your worlds or your WooHoos.

Screenshots of these help too:

- **Avatars > Edit > What this avatar can do**, for the avatar with the problem
- The link details at the top of **Settings > VRChat**
- Your app version, from **Settings > Updates**

Also say whether you uploaded the avatar recently, and whether **Reset Config** changed anything.
