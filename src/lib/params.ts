// Friendly names and explanations for the app's own avatar parameters.

export const PARAM_ORDER = ["stage", "bellyExtra", "appActive", "appHeartbeat", "fertileLevel", "waiting", "babyCount", "fetalMovement", "controlStart", "controlPause", "controlStop"];

export const PARAM_INFO: Record<string, { label: string; text: string }> = {
  stage: { label: "Belly size", text: "0 to 1. The app writes this. Required." },
  bellyExtra: { label: "Extra size", text: "0 to 1. Bigger for twins and more, and when overdue. For the SDK's Bigger Belly." },
  appActive: { label: "App is running", text: "True while the app runs, so the avatar can pause its own counter." },
  appHeartbeat: { label: "Heartbeat", text: "Flips every second. If it stops, the app has closed." },
  fertileLevel: { label: "Fertile level", text: "0 low, 1 medium, 2 high, 3 peak." },
  waiting: { label: "Waiting", text: "True during the grace period after a WooHoo." },
  babyCount: { label: "Baby count", text: "0 until it can be seen (week 12), then 1 to 4." },
  fetalMovement: { label: "Fetal movement", text: "Kicks: the values sent are set under Kick values below." },
  controlStart: { label: "Menu: Start", text: "Button in the avatar's App Controls menu. Starts a pregnancy." },
  controlPause: { label: "Menu: Pause", text: "Button in the avatar's App Controls menu. Pauses or resumes." },
  controlStop: { label: "Menu: Stop", text: "Button in the avatar's App Controls menu. Hold 2 seconds to end the pregnancy." },
};
