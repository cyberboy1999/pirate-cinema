package dev.dioxus.main

import android.content.Intent

typealias BuildConfig = com.piratecinema.rust.BuildConfig

class MainActivity : WryActivity() {
    private external fun playerResult(position: Long, duration: Long, completed: Boolean, source: String?)

    @Deprecated("Android activity result bridge")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != 7001) return
        val position = if (data?.hasExtra("position") == true) data.getIntExtra("position", -1).toLong() else -1L
        val duration = if (data?.hasExtra("duration") == true) data.getIntExtra("duration", -1).toLong() else -1L
        // Only mpv's successful, nonempty EOF reply can indicate completion.
        playerResult(position, duration, resultCode == RESULT_OK && data != null && !data.hasExtra("position"), data?.data?.toString())
    }
}
