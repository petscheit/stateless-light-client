import pandas as pd
import matplotlib.pyplot as plt
import os
import numpy as np
import math

# --- Create a directory for diagrams ---
DIAGRAMS_DIR = 'diagrams'
os.makedirs(DIAGRAMS_DIR, exist_ok=True)

# --- load data from benchmark CSV ------------------------------------
csv_path = os.path.join(os.path.dirname(__file__), 'bankai_metrics.csv')
df_raw = pd.read_csv(csv_path)

# Shape into the columns expected below
df = df_raw[["epoch", "n_steps", "time"]].rename(columns={"time": "proving_time"}).copy()
df = df.dropna(subset=["epoch", "n_steps", "proving_time"]).sort_values("epoch").reset_index(drop=True)

# --------------------------------------------------------------------
# 1)  Step‑count vs epoch
plt.figure(figsize=(10, 5))
plt.plot(df["epoch"], df["n_steps"], marker="o", linewidth=2)
plt.xlabel("Epoch")
plt.ylabel("Total Cairo VM steps")
plt.title("Total Step Count per Epoch")
plt.xticks(df["epoch"][::2], rotation=45)
plt.grid(True, linestyle="--", linewidth=0.5)
plt.tight_layout()
plt.savefig(os.path.join(DIAGRAMS_DIR, "step_count_vs_epoch.png"))
plt.show()

# --------------------------------------------------------------------
# 2)  Proving‑time vs epoch
plt.figure(figsize=(10, 5))
plt.plot(df["epoch"], df["proving_time"], marker="o", linewidth=2)
plt.xlabel("Epoch")
plt.ylabel("Proving Time (seconds)")
plt.title("Proving Time per Epoch")
plt.xticks(df["epoch"][::2], rotation=45)
plt.grid(True, linestyle="--", linewidth=0.5)
plt.tight_layout()
plt.savefig(os.path.join(DIAGRAMS_DIR, "proving_time_vs_epoch.png"))
plt.show()

# --------------------------------------------------------------------
# 3) Proving Time Distribution
plt.figure(figsize=(8, 6))
plt.boxplot(df["proving_time"], vert=False)
plt.xlabel("Proving Time (seconds)")
plt.title("Distribution of Proving Times")
plt.yticks([])  # We don't need a y-tick for a single box plot
plt.grid(True, linestyle="--", linewidth=0.5)
plt.tight_layout()
plt.savefig(os.path.join(DIAGRAMS_DIR, "proving_time_distribution.png"))
plt.show()

# --------------------------------------------------------------------
# 4) Recursive-counter step counts (raw)
# Data from the screenshot
rounds = list(range(1, 10))
steps = [11, 2322005, 2832636, 2714132, 2718837, 2721987, 2727275, 2719123, 2713958]

# Create a DataFrame
df_rounds = pd.DataFrame({"Round": rounds, "Steps": steps})

# ---- Figure 3: Recursive‑counter step counts ----
plt.figure(figsize=(10, 5))
plt.plot(df_rounds["Round"], df_rounds["Steps"], marker="o", linewidth=2, label="Observed")
plt.xlabel("Round")
plt.ylabel("Total Cairo VM steps")
plt.title("Recursive‑Counter: Steps per Round")
plt.xticks(df_rounds["Round"])
plt.grid(True, linestyle="--", linewidth=0.5)
plt.tight_layout()
plt.savefig(os.path.join(DIAGRAMS_DIR, "recursive_counter_steps_per_round.png"))
plt.show()

# --------------------------------------------------------------------
# 5) Recursive-counter Observed vs. Predicted
# ----- Derive an approximate formula: Steps_n ≈ C + α * ln(Steps_{n-1}) -----
C = min(steps[1:])  # baseline recursion overhead (~2.322M)
xs = [math.log(steps[i-1]) for i in range(2, len(steps))]
ys = [steps[i] - C for i in range(2, len(steps))]
alpha = sum(x * y for x, y in zip(xs, ys)) / sum(x * x for x in xs)

print(f"\nDerived alpha for recursive-counter formula: {alpha:.2f}")

# Generate predicted values using the fitted formula
predicted = [steps[0], steps[1]]  # seed with first two
for n in range(2, len(steps)):
    predicted.append(int(C + alpha * math.log(predicted[-1])))

# Plot observed vs predicted
plt.figure(figsize=(10, 5))
plt.plot(df_rounds["Round"], df_rounds["Steps"], marker="o", linewidth=2, label="Observed")
plt.plot(df_rounds["Round"], predicted, marker="s", linestyle="--", linewidth=2, label="Predicted")
plt.xlabel("Round")
plt.ylabel("Total Cairo VM steps")
plt.title("Recursive‑Counter: Observed vs Predicted")
plt.xticks(df_rounds["Round"])
plt.grid(True, linestyle="--", linewidth=0.5)
plt.legend()
plt.tight_layout()
plt.savefig(os.path.join(DIAGRAMS_DIR, "recursive_counter_observed_vs_predicted.png"))
plt.show()

# --------------------------------------------------------------------
# 6) Light-client Observed vs. Predicted
# --- Reuse C from recursive counter, derive a new alpha for light-client steps ---
lc_steps = df["n_steps"].tolist()

# Use C from the recursive-counter data (steps[0] is an outlier)
# C = min(steps[1:]) which is 2322005

# For light-client, our model is: steps_n ≈ C + α * ln(steps_{n-1})
# We derive alpha using linear regression (y = a*x, where y = steps_n - C, x = ln(steps_{n-1}))
lc_xs = [math.log(lc_steps[i - 1]) for i in range(1, len(lc_steps))]
lc_ys = [lc_steps[i] - C for i in range(1, len(lc_steps))]
lc_alpha = sum(x * y for x, y in zip(lc_xs, lc_ys)) / sum(x * x for x in lc_xs)

print(f"Derived alpha for light-client formula: {lc_alpha:.2f}")

# --- Generate predicted values using the fitted formula ---
# Seed with the first observed value, then predict subsequent steps
lc_predicted = [lc_steps[0]]
for _ in range(1, len(lc_steps)):
    next_step = int(C + lc_alpha * math.log(lc_predicted[-1]))
    lc_predicted.append(next_step)

# --- Plot observed vs predicted for light-client ---
lc_rounds = range(1, len(lc_steps) + 1)
plt.figure(figsize=(10, 5))
plt.plot(lc_rounds, lc_steps, marker="o", linewidth=2, label="Observed")
plt.plot(lc_rounds, lc_predicted, marker="s", linestyle="--", linewidth=2, label="Predicted")
plt.xlabel("Round")
plt.ylabel("Total Cairo VM steps")
plt.title("Light-Client: Observed vs Predicted")
plt.xticks(rotation=45)
plt.grid(True, linestyle="--", linewidth=0.5)
plt.legend()
plt.tight_layout()
plt.savefig(os.path.join(DIAGRAMS_DIR, "light_client_observed_vs_predicted.png"))
plt.show()

# --------------------------------------------------------------------
# 7) Light-client Observed steps
# --- This is a simplified version of the above, showing only measured data ---

# --- Plot observed data for light-client ---
plt.figure(figsize=(10, 5))
plt.plot(df["epoch"], df["n_steps"], marker="o", linewidth=2)
plt.xlabel("Epoch")
plt.ylabel("Total Cairo VM steps")
plt.title("Light-Client: Observed Steps per Epoch")
plt.xticks(df["epoch"][::2], rotation=45)
plt.grid(True, linestyle="--", linewidth=0.5)
plt.tight_layout()
plt.savefig(os.path.join(DIAGRAMS_DIR, "light_client_observed_steps.png"))
plt.show()
