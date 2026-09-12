package ai.mobileqa.demo;

import android.app.Activity;
import android.os.Bundle;
import android.content.SharedPreferences;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.view.inputmethod.InputMethodManager;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** Controlled fixture: identical UI, with only durable storage changed by build flavor. */
public final class MainActivity extends Activity {
    private final ExecutorService network = Executors.newSingleThreadExecutor();
    private LinearLayout layout;

    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        layout = new LinearLayout(this);
        layout.setOrientation(LinearLayout.VERTICAL);
        layout.setPadding(32, 80, 32, 32);
        setContentView(layout);
        label("Checking test session…", 0);
        network.execute(() -> {
            boolean ready = false;
            HttpURLConnection connection = null;
            try {
                // The runner establishes an ADB reverse tunnel before launch. This
                // fixture must not depend on cold-boot Wi-Fi initialization timing.
                connection = (HttpURLConnection) new URL("http://127.0.0.1:8765/session").openConnection();
                connection.setConnectTimeout(3000);
                connection.setReadTimeout(3000);
                ready = connection.getResponseCode() == 200;
            } catch (Exception ignored) {
                // A missing fixture prerequisite has a distinct screen, never a saved task.
            } finally { if (connection != null) connection.disconnect(); }
            final boolean available = ready;
            runOnUiThread(() -> {
                if (isFinishing() || isDestroyed()) return;
                layout.removeAllViews();
                if (available) showTasks();
                else label("Test session unavailable", R.id.prerequisite_unavailable);
            });
        });
    }

    private void label(String text, int id) {
        TextView label = new TextView(this);
        if (id != 0) label.setId(id);
        label.setText(text);
        label.setTextSize(22);
        layout.addView(label);
    }

    private void showTasks() {
        label("Tasks ready", R.id.ready_marker);
        EditText input = new EditText(this);
        input.setId(R.id.task_input);
        input.setHint("Task name");
        input.setSingleLine(true);
        layout.addView(input);
        Button save = new Button(this);
        save.setId(R.id.save_task);
        save.setText("Save");
        layout.addView(save);
        LinearLayout tasks = new LinearLayout(this);
        tasks.setOrientation(LinearLayout.VERTICAL);
        tasks.setId(R.id.task_list);
        layout.addView(tasks);
        SharedPreferences storage = getSharedPreferences("tasks", MODE_PRIVATE);
        String existing = BuildConfig.PERSIST ? storage.getString("task", "") : "";
        if (!existing.isEmpty()) addRow(tasks, existing);
        save.setOnClickListener(view -> {
            String task = input.getText().toString();
            if (task.isEmpty()) return;
            // Deliberate broken flavor displays a row but omits the durable commit.
            if (BuildConfig.PERSIST && !storage.edit().putString("task", task).commit()) return;
            tasks.removeAllViews();
            addRow(tasks, task);
            input.setText("");
            ((InputMethodManager)getSystemService(INPUT_METHOD_SERVICE)).hideSoftInputFromWindow(input.getWindowToken(), 0);
            input.clearFocus();
        });
    }

    private void addRow(LinearLayout tasks, String task) {
        TextView row = new TextView(this);
        row.setId(R.id.task_row);
        row.setText(task);
        row.setTextSize(20);
        tasks.addView(row);
    }

    @Override public void onDestroy() { network.shutdownNow(); super.onDestroy(); }
}
