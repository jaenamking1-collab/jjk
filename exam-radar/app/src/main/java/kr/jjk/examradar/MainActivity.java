package kr.jjk.examradar;

import android.Manifest;
import android.animation.ArgbEvaluator;
import android.animation.ValueAnimator;
import android.app.Activity;
import android.app.AlertDialog;
import android.bluetooth.BluetoothAdapter;
import android.bluetooth.BluetoothManager;
import android.bluetooth.le.BluetoothLeScanner;
import android.bluetooth.le.ScanCallback;
import android.bluetooth.le.ScanRecord;
import android.bluetooth.le.ScanResult;
import android.bluetooth.le.ScanSettings;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.location.LocationManager;
import android.media.AudioManager;
import android.media.ToneGenerator;
import android.net.wifi.WifiManager;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import android.os.VibrationEffect;
import android.os.Vibrator;
import android.provider.Settings;
import android.util.SparseArray;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;

/**
 * 시험감독 레이더 (베타)
 * 1) 기준 잡기: 빈 교실에서 30초 동안 보이는 와이파이·블루투스 신호를 '원래 있던 것'으로 저장
 * 2) 감시: 기준에 없던 신호가 가까이 나타나면 화면이 깜빡이고 진동
 * 3) 찾기: 항목을 누르면 대략 거리와 가까워짐/멀어짐을 실시간으로 표시
 */
public class MainActivity extends Activity {

    // 색: 디자인.md '대비 강한 조합' + 경고는 쨍한 오렌지
    static final int NAVY = 0xFF111827, NAVY2 = 0xFF1F2A44, WHITE = 0xFFF3F4F6, LIME = 0xFFA3E635,
            ORANGE = 0xFFFF4C02, SKY = 0xFF60A5FA, DIM = 0xFF8FA3C7;

    static final int MODE_IDLE = 0, MODE_BASELINE = 1, MODE_WATCH = 2;
    static final int BASELINE_SECONDS = 30;
    static final long GONE_MS = 45_000;      // 이만큼 안 보이면 사라진 것으로
    static final int REQ_PERM = 7;

    static class Dev {
        String key, name, kind;   // kind: 기기 종류 설명
        boolean wifi, phoneLike;
        int manu = -1;            // 블루투스 제조사 번호
        int rssi;
        double ema = Double.NaN;
        long firstSeen, lastSeen;
        int hits;
        final ArrayList<double[]> hist = new ArrayList<>(); // [시각, ema]
    }

    final Map<String, Dev> devs = new HashMap<>();
    final Set<String> baseline = new HashSet<>();
    final Set<String> acked = new HashSet<>();   // 경고 확인을 누른 새 기기
    int mode = MODE_IDLE;
    long baselineEnd;
    boolean wide = false, sound = false;
    String findKey = null;
    long lastWifiScanOk = 0, lastWifiResult = 0, bleStartedAt = 0;
    boolean wifiThrottled = false;

    SharedPreferences prefs;
    WifiManager wifi;
    BluetoothLeScanner ble;
    final Handler h = new Handler(Looper.getMainLooper());
    ToneGenerator tone;
    Vibrator vib;
    ValueAnimator flash;
    long lastBeep = 0, lastVibe = 0;

    // 화면 요소
    FrameLayout root;
    LinearLayout main, findView;
    TextView status, statusSub, baseInfo, tip, findName, findDist, findTrend, findRaw;
    View findBarFill;
    LinearLayout list;
    Button btnBase, btnWatch, btnRange, btnSound, btnAck;

    // ───────────────────────── 생명주기 ─────────────────────────

    @Override
    protected void onCreate(Bundle b) {
        super.onCreate(b);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        getWindow().setStatusBarColor(NAVY);
        getWindow().setNavigationBarColor(NAVY);
        prefs = getSharedPreferences("radar", MODE_PRIVATE);
        baseline.addAll(prefs.getStringSet("baseline", Collections.emptySet()));
        wide = prefs.getBoolean("wide", false);
        sound = prefs.getBoolean("sound", false);
        wifi = (WifiManager) getApplicationContext().getSystemService(Context.WIFI_SERVICE);
        vib = (Vibrator) getSystemService(Context.VIBRATOR_SERVICE);
        try { tone = new ToneGenerator(AudioManager.STREAM_MUSIC, 80); } catch (Exception ignored) { }
        buildUi();
        askPermissions();
    }

    @Override
    protected void onResume() {
        super.onResume();
        if (hasPerms()) startScanning();
        h.removeCallbacks(tick);
        h.post(tick);
    }

    @Override
    protected void onPause() {
        super.onPause();
        // 감시 중이면 화면이 켜져 있는 동안 계속 돈다. 앱을 벗어나면 검색을 멈춘다.
        stopScanning();
        h.removeCallbacks(tick);
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        if (tone != null) tone.release();
    }

    @Override
    public void onBackPressed() {
        if (findKey != null) { closeFinder(); return; }
        super.onBackPressed();
    }

    // ───────────────────────── 권한 ─────────────────────────

    String[] neededPerms() {
        List<String> p = new ArrayList<>();
        p.add(Manifest.permission.ACCESS_FINE_LOCATION);
        p.add(Manifest.permission.ACCESS_COARSE_LOCATION);
        if (Build.VERSION.SDK_INT >= 31) p.add(Manifest.permission.BLUETOOTH_SCAN);
        if (Build.VERSION.SDK_INT >= 33) p.add(Manifest.permission.NEARBY_WIFI_DEVICES);
        return p.toArray(new String[0]);
    }

    boolean hasPerms() {
        for (String s : neededPerms())
            if (checkSelfPermission(s) != PackageManager.PERMISSION_GRANTED
                    && !s.equals(Manifest.permission.ACCESS_COARSE_LOCATION)) return false;
        return true;
    }

    void askPermissions() {
        if (hasPerms()) { checkSwitches(); return; }
        new AlertDialog.Builder(this)
                .setTitle("권한이 필요합니다")
                .setMessage("주변 와이파이·블루투스 신호를 보려면 '위치'와 '근처 기기' 권한이 필요합니다.\n\n"
                        + "위치는 신호 검색에만 쓰고, 어디에도 저장하거나 보내지 않습니다.")
                .setPositiveButton("허용하러 가기", (d, w) -> requestPermissions(neededPerms(), REQ_PERM))
                .setCancelable(false)
                .show();
    }

    @Override
    public void onRequestPermissionsResult(int code, String[] perms, int[] res) {
        if (code != REQ_PERM) return;
        if (hasPerms()) { checkSwitches(); startScanning(); }
        else {
            new AlertDialog.Builder(this)
                    .setTitle("권한이 꺼져 있습니다")
                    .setMessage("설정 → 애플리케이션 → 시험감독 레이더 → 권한에서 '위치'(정확한 위치)와 '근처 기기'를 허용해 주세요.")
                    .setPositiveButton("설정 열기", (d, w) -> {
                        Intent i = new Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS);
                        i.setData(android.net.Uri.parse("package:" + getPackageName()));
                        startActivity(i);
                    })
                    .setNegativeButton("닫기", null).show();
        }
    }

    /** 위치·와이파이·블루투스 스위치가 꺼져 있으면 알려 준다 */
    void checkSwitches() {
        List<String> off = new ArrayList<>();
        LocationManager lm = (LocationManager) getSystemService(Context.LOCATION_SERVICE);
        boolean locOn = Build.VERSION.SDK_INT >= 28 ? lm.isLocationEnabled()
                : lm.isProviderEnabled(LocationManager.NETWORK_PROVIDER) || lm.isProviderEnabled(LocationManager.GPS_PROVIDER);
        if (!locOn) off.add("위치");
        if (wifi != null && !wifi.isWifiEnabled()) off.add("와이파이");
        BluetoothAdapter ad = btAdapter();
        if (ad == null || !ad.isEnabled()) off.add("블루투스");
        if (off.isEmpty()) return;
        String what = String.join(", ", off);
        String target = off.get(0).equals("위치") ? Settings.ACTION_LOCATION_SOURCE_SETTINGS
                : off.get(0).equals("와이파이") ? Settings.ACTION_WIFI_SETTINGS : Settings.ACTION_BLUETOOTH_SETTINGS;
        new AlertDialog.Builder(this)
                .setTitle(what + " 꺼짐")
                .setMessage(what + "을(를) 켜야 신호가 보입니다.\n와이파이는 어디에도 연결하지 않아도 됩니다. 스위치만 켜 두세요.")
                .setPositiveButton("켜러 가기", (d, w) -> startActivity(new Intent(target)))
                .setNegativeButton("나중에", null).show();
    }

    BluetoothAdapter btAdapter() {
        BluetoothManager bm = (BluetoothManager) getSystemService(Context.BLUETOOTH_SERVICE);
        return bm == null ? null : bm.getAdapter();
    }

    // ───────────────────────── 검색 ─────────────────────────

    final BroadcastReceiver wifiRx = new BroadcastReceiver() {
        @Override public void onReceive(Context c, Intent i) { readWifi(); }
    };
    boolean wifiRxOn = false;

    void startScanning() {
        if (!wifiRxOn) {
            registerReceiver(wifiRx, new IntentFilter(WifiManager.SCAN_RESULTS_AVAILABLE_ACTION));
            wifiRxOn = true;
        }
        startBle();
    }

    void stopScanning() {
        if (wifiRxOn) { try { unregisterReceiver(wifiRx); } catch (Exception ignored) { } wifiRxOn = false; }
        stopBle();
    }

    @SuppressWarnings("deprecation")
    void kickWifiScan() {
        if (wifi == null) return;
        try {
            boolean ok = wifi.startScan();
            if (ok) { lastWifiScanOk = System.currentTimeMillis(); wifiThrottled = false; }
            else wifiThrottled = true;
        } catch (SecurityException ignored) { }
        readWifi();
    }

    void readWifi() {
        if (wifi == null) return;
        List<android.net.wifi.ScanResult> rs;
        try { rs = wifi.getScanResults(); } catch (SecurityException e) { return; }
        if (rs == null) return;
        long nowUs = SystemClock.elapsedRealtime() * 1000L;
        long now = System.currentTimeMillis();
        for (android.net.wifi.ScanResult r : rs) {
            if (nowUs - r.timestamp > 40_000_000L) continue; // 40초보다 오래된 결과는 버림
            String key = "W:" + r.BSSID;
            Dev d = devs.get(key);
            long seenAt = now - (nowUs - r.timestamp) / 1000L;
            if (d == null) {
                d = new Dev(); d.key = key; d.wifi = true; d.firstSeen = seenAt; devs.put(key, d);
            }
            if (seenAt <= d.lastSeen) continue; // 같은 결과를 두 번 세지 않음
            String ssid = ssidOf(r);
            d.name = ssid.isEmpty() ? "(이름 숨김)" : ssid;
            d.phoneLike = looksLikePhone(ssid, r);
            d.kind = (d.phoneLike ? "휴대폰 핫스팟 의심 · " : "와이파이 · ") + (r.frequency >= 5900 ? "6GHz" : r.frequency >= 4900 ? "5GHz" : "2.4GHz");
            feed(d, r.level, seenAt);
            lastWifiResult = now;
        }
    }

    @SuppressWarnings("deprecation")
    static String ssidOf(android.net.wifi.ScanResult r) {
        if (Build.VERSION.SDK_INT >= 33 && r.getWifiSsid() != null) {
            String s = r.getWifiSsid().toString();
            if (s.startsWith("\"") && s.endsWith("\"") && s.length() >= 2) return s.substring(1, s.length() - 1);
            return s.equals("<unknown ssid>") ? "" : s;
        }
        return r.SSID == null ? "" : r.SSID;
    }

    static boolean looksLikePhone(String ssid, android.net.wifi.ScanResult r) {
        String s = ssid.toLowerCase(Locale.ROOT);
        String[] hints = {"iphone", "galaxy", "androidap", "android", "redmi", "xiaomi", "pixel", "note", "의 ", "hotspot", "핫스팟", "phone", "sm-"};
        for (String k : hints) if (s.contains(k)) return true;
        // 휴대폰 핫스팟은 보통 '임의 주소'(두 번째 자리 비트가 켜짐)를 쓴다
        try {
            int b0 = Integer.parseInt(r.BSSID.substring(0, 2), 16);
            if ((b0 & 0x02) != 0) return true;
        } catch (Exception ignored) { }
        return false;
    }

    final ScanCallback bleCb = new ScanCallback() {
        @Override public void onScanResult(int type, ScanResult r) { onBle(r); }
        @Override public void onBatchScanResults(List<ScanResult> rs) { for (ScanResult r : rs) onBle(r); }
    };

    void startBle() {
        BluetoothAdapter ad = btAdapter();
        if (ad == null || !ad.isEnabled()) return;
        try {
            ble = ad.getBluetoothLeScanner();
            if (ble == null) return;
            ScanSettings s = new ScanSettings.Builder()
                    .setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY)
                    .setReportDelay(0).build();
            ble.startScan(null, s, bleCb);
            bleStartedAt = System.currentTimeMillis();
        } catch (SecurityException | IllegalStateException ignored) { }
    }

    void stopBle() {
        if (ble == null) return;
        try { ble.stopScan(bleCb); } catch (Exception ignored) { }
        ble = null;
    }

    void onBle(ScanResult r) {
        String key = "B:" + r.getDevice().getAddress();
        long now = System.currentTimeMillis();
        Dev d = devs.get(key);
        boolean fresh = d == null;
        if (fresh) { d = new Dev(); d.key = key; d.wifi = false; d.firstSeen = now; devs.put(key, d); }
        ScanRecord rec = r.getScanRecord();
        String nm = rec == null ? null : rec.getDeviceName();
        int manu = -1;
        if (rec != null) {
            SparseArray<byte[]> m = rec.getManufacturerSpecificData();
            if (m != null && m.size() > 0) manu = m.keyAt(0);
        }
        d.manu = manu;
        String maker = maker(manu);
        d.name = (nm != null && !nm.trim().isEmpty()) ? nm.trim() : (maker != null ? maker + " 기기" : "이름 없는 기기");
        d.phoneLike = manu == 0x004C || manu == 0x0075 || manu == 0x00E0;
        d.kind = "블루투스" + (maker != null ? " · " + maker : "");
        feed(d, r.getRssi(), now);
        if (fresh && mode == MODE_WATCH) absorbRotation(d);
    }

    static String maker(int id) {
        switch (id) {
            case 0x004C: return "애플";
            case 0x0075: return "삼성";
            case 0x00E0: return "구글";
            case 0x0006: return "마이크로소프트";
            case 0x038F: case 0x0157: return "샤오미";
            case 0x0087: return "가민";
            case 0x0059: return "노르딕";
            case 0x012D: return "소니";
            case 0x00D2: return "LG";
            default: return null;
        }
    }

    /**
     * 요즘 폰·이어폰은 개인정보 보호로 블루투스 주소를 주기적으로 바꾼다.
     * 새로 나타난 기기와 같은 제조사의 '기준 기기'가 방금 사라졌고 세기도 비슷하면 같은 기기로 보고 기준에 넣는다.
     */
    void absorbRotation(Dev nd) {
        if (nd.manu < 0) return;
        long now = System.currentTimeMillis();
        for (Dev o : devs.values()) {
            if (o == nd || o.wifi || o.manu != nd.manu) continue;
            if (!baseline.contains(o.key)) continue;
            long gap = now - o.lastSeen;
            if (gap < 2_000 || gap > 120_000) continue;
            if (!Double.isNaN(o.ema) && Math.abs(o.ema - nd.rssi) <= 12) {
                baseline.add(nd.key);
                saveBaseline();
                return;
            }
        }
    }

    void feed(Dev d, int rssi, long at) {
        d.rssi = rssi;
        d.ema = Double.isNaN(d.ema) ? rssi : d.ema * 0.6 + rssi * 0.4;
        d.lastSeen = Math.max(d.lastSeen, at);
        d.hits++;
        d.hist.add(new double[]{at, d.ema});
        while (d.hist.size() > 40) d.hist.remove(0);
        if (mode == MODE_BASELINE) baseline.add(d.key);
    }

    // ───────────────────────── 판단 ─────────────────────────

    int wifiLimit() { return wide ? -88 : -75; }
    int bleLimit() { return wide ? -85 : -72; }

    boolean isAlert(Dev d, long now) {
        if (mode != MODE_WATCH || baseline.contains(d.key)) return false;
        if (now - d.lastSeen > GONE_MS) return false;
        if (d.wifi) return d.ema >= wifiLimit();
        // 블루투스는 3번 이상, 3초 이상 보이고 가까울 때만 (스쳐 가는 신호 거르기)
        return d.hits >= 3 && now - d.firstSeen >= 3_000 && d.ema >= bleLimit();
    }

    /** 신호 세기로 대략 거리(m). 실내에서는 2~3배 오차가 흔하다. */
    static double meters(Dev d) {
        double ref = d.wifi ? -40 : -60;   // 1m 에서의 대략 세기
        double n = d.wifi ? 2.7 : 2.2;     // 실내 감쇠
        double m = Math.pow(10, (ref - d.ema) / (10 * n));
        return Math.max(0.3, Math.min(30, m));
    }

    static String distText(double m) {
        if (m < 1) return "1m 안";
        if (m < 10) return String.format(Locale.KOREA, "약 %.0fm", m);
        return "약 " + (Math.round(m / 5.0) * 5) + "m";
    }

    static String zone(double m) {
        if (m < 1) return "바로 옆";
        if (m < 3) return "가까움";
        if (m < 8) return "같은 교실";
        return "멀리 (복도·옆 반일 수 있음)";
    }

    /** 최근 3초 사이 세기 변화: +면 가까워짐 */
    static double trend(Dev d) {
        if (d.hist.size() < 2) return 0;
        double[] last = d.hist.get(d.hist.size() - 1);
        for (int i = d.hist.size() - 2; i >= 0; i--) {
            double[] p = d.hist.get(i);
            if (last[0] - p[0] >= 3000) return last[1] - p[1];
        }
        return last[1] - d.hist.get(0)[1];
    }

    // ───────────────────────── 1초마다 ─────────────────────────

    int tickN = 0;
    final Runnable tick = new Runnable() {
        @Override public void run() {
            tickN++;
            long now = System.currentTimeMillis();
            if (tickN % 4 == 1) kickWifiScan();
            // 안드로이드는 30분 넘게 돈 블루투스 검색을 느리게 만든다 → 20분마다 다시 시작
            if (ble != null && now - bleStartedAt > 20 * 60_000L) { stopBle(); startBle(); }
            if (ble == null && hasPerms() && tickN % 5 == 0) startBle();
            if (mode == MODE_BASELINE && now >= baselineEnd) finishBaseline();
            // 오래 안 보인 항목 정리
            devs.values().removeIf(d -> now - d.lastSeen > 10 * 60_000L);
            render(now);
            h.postDelayed(this, 1000);
        }
    };

    // ───────────────────────── 동작 ─────────────────────────

    void startBaseline() {
        if (!hasPerms()) { askPermissions(); return; }
        new AlertDialog.Builder(this)
                .setTitle("기준 잡기")
                .setMessage("학생이 들어오기 전, 빈 교실에서 해 주세요.\n\n30초 동안 지금 보이는 신호를 모두 '원래 있던 것'으로 저장합니다. 이전 기준은 지워집니다.\n\n감독관 본인 휴대폰은 이때 켜 두면 경고에서 빠집니다.")
                .setPositiveButton("시작", (d, w) -> {
                    baseline.clear(); acked.clear(); devs.clear();
                    mode = MODE_BASELINE;
                    baselineEnd = System.currentTimeMillis() + BASELINE_SECONDS * 1000L;
                    stopBle(); startBle(); kickWifiScan();
                    render(System.currentTimeMillis());
                })
                .setNegativeButton("취소", null).show();
    }

    void finishBaseline() {
        long now = System.currentTimeMillis();
        for (Dev d : devs.values()) if (now - d.lastSeen < GONE_MS) baseline.add(d.key);
        saveBaseline();
        mode = MODE_WATCH;
        buzz(new long[]{0, 120, 80, 120});
    }

    void toggleWatch() {
        if (mode == MODE_BASELINE) return;
        if (mode == MODE_WATCH) { mode = MODE_IDLE; stopFlash(); return; }
        if (baseline.isEmpty()) { startBaseline(); return; }
        if (!hasPerms()) { askPermissions(); return; }
        mode = MODE_WATCH;
    }

    void ignore(Dev d) {
        baseline.add(d.key);
        saveBaseline();
        if (d.key.equals(findKey)) closeFinder();
    }

    void ackAll() {
        long now = System.currentTimeMillis();
        for (Dev d : devs.values()) if (isAlert(d, now)) acked.add(d.key);
        stopFlash();
    }

    void saveBaseline() { prefs.edit().putStringSet("baseline", new HashSet<>(baseline)).apply(); }

    void buzz(long[] pattern) {
        if (vib == null || !vib.hasVibrator()) return;
        vib.vibrate(VibrationEffect.createWaveform(pattern, -1));
    }

    void startFlash() {
        if (flash != null && flash.isRunning()) return;
        flash = ValueAnimator.ofObject(new ArgbEvaluator(), ORANGE, NAVY2);
        flash.setDuration(450);
        flash.setRepeatMode(ValueAnimator.REVERSE);
        flash.setRepeatCount(ValueAnimator.INFINITE);
        flash.addUpdateListener(a -> statusCard.setBackground(round((int) a.getAnimatedValue(), 18)));
        flash.start();
    }

    void stopFlash() {
        if (flash != null) flash.cancel();
        flash = null;
        if (statusCard != null) statusCard.setBackground(round(NAVY2, 18));
    }

    // ───────────────────────── 화면 그리기 ─────────────────────────

    LinearLayout statusCard;

    void render(long now) {
        if (findKey != null) { renderFinder(now); return; }
        List<Dev> alerts = new ArrayList<>();
        int baseSeen = 0;
        for (Dev d : devs.values()) {
            if (isAlert(d, now)) alerts.add(d);
            else if (baseline.contains(d.key) && now - d.lastSeen < GONE_MS) baseSeen++;
        }
        alerts.sort((a, b) -> Double.compare(b.ema, a.ema));
        int unacked = 0;
        for (Dev d : alerts) if (!acked.contains(d.key)) unacked++;

        if (mode == MODE_BASELINE) {
            long left = Math.max(0, (baselineEnd - now + 999) / 1000);
            status.setText("기준 잡는 중 · " + left + "초");
            status.setTextColor(LIME);
            statusSub.setText("지금 보이는 신호 " + baseline.size() + "개를 기억하고 있습니다. 태블릿을 교실 가운데쯤 두세요.");
            stopFlash();
        } else if (mode == MODE_WATCH) {
            if (alerts.isEmpty()) {
                status.setText("감시 중 · 이상 없음");
                status.setTextColor(LIME);
                statusSub.setText("새로 나타난 기기가 없습니다.");
                stopFlash();
            } else {
                status.setText("새 기기 " + alerts.size() + "개");
                status.setTextColor(WHITE);
                statusSub.setText(unacked > 0 ? "목록에서 눌러 거리를 보며 찾으세요." : "확인함. 새로 생기면 다시 깜빡입니다.");
                if (unacked > 0) {
                    startFlash();
                    if (now - lastVibe > 6000) { buzz(new long[]{0, 300, 150, 300}); lastVibe = now; }
                    if (sound && tone != null && now - lastBeep > 6000) { tone.startTone(ToneGenerator.TONE_PROP_BEEP2, 300); lastBeep = now; }
                } else stopFlash();
            }
        } else {
            status.setText(baseline.isEmpty() ? "준비" : "멈춤");
            status.setTextColor(WHITE);
            statusSub.setText(baseline.isEmpty() ? "① 빈 교실에서 '기준 잡기'를 누르세요." : "'감시 시작'을 누르면 새 기기를 찾기 시작합니다.");
            stopFlash();
        }
        btnAck.setVisibility(mode == MODE_WATCH && unacked > 0 ? View.VISIBLE : View.GONE);
        btnBase.setEnabled(mode != MODE_BASELINE);
        btnWatch.setText(mode == MODE_WATCH ? "감시 멈춤" : "감시 시작");
        styleBtn(btnWatch, mode == MODE_WATCH ? NAVY2 : LIME, mode == MODE_WATCH ? WHITE : NAVY);
        btnRange.setText(wide ? "경고 범위: 넓게" : "경고 범위: 교실 안");
        btnSound.setText(sound ? "소리: 켬" : "소리: 끔 (진동만)");

        baseInfo.setText("기준 기기 " + baseline.size() + "개 저장됨 · 지금 보이는 기준 기기 " + baseSeen + "개");
        String t = "";
        if (wifiThrottled && now - lastWifiScanOk > 20_000)
            t = "와이파이 검색이 느립니다. 설정 → 개발자 옵션 → 'Wi-Fi 검색 제한'을 끄면 4초마다 갱신됩니다.";
        else if (lastWifiResult > 0 && now - lastWifiResult > 60_000)
            t = "1분 넘게 와이파이 결과가 없습니다. 와이파이 스위치와 위치가 켜져 있는지 보세요.";
        tip.setText(t);
        tip.setVisibility(t.isEmpty() ? View.GONE : View.VISIBLE);

        list.removeAllViews();
        if (mode == MODE_WATCH && alerts.isEmpty()) {
            list.addView(label("새 기기가 나타나면 여기에 뜹니다.", 15, DIM, false));
        }
        for (Dev d : alerts) list.addView(row(d, now));
    }

    View row(Dev d, long now) {
        LinearLayout r = new LinearLayout(this);
        r.setOrientation(LinearLayout.HORIZONTAL);
        r.setGravity(Gravity.CENTER_VERTICAL);
        int pad = dp(14);
        r.setPadding(pad, pad, pad, pad);
        boolean isNew = !acked.contains(d.key);
        r.setBackground(round(NAVY2, 14, isNew ? ORANGE : (d.wifi ? SKY : LIME)));
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(-1, -2);
        lp.bottomMargin = dp(10);
        r.setLayoutParams(lp);

        TextView badge = label(d.wifi ? "와이파이" : "블루투스", 13, NAVY, true);
        badge.setBackground(round(d.wifi ? SKY : LIME, 8));
        badge.setPadding(dp(8), dp(4), dp(8), dp(4));
        r.addView(badge);

        LinearLayout mid = new LinearLayout(this);
        mid.setOrientation(LinearLayout.VERTICAL);
        mid.setPadding(dp(12), 0, dp(8), 0);
        mid.addView(label(d.name, 18, WHITE, true));
        long ago = (now - d.lastSeen) / 1000;
        mid.addView(label(d.kind + (ago > 5 ? " · " + ago + "초 전" : ""), 13, DIM, false));
        r.addView(mid, new LinearLayout.LayoutParams(0, -2, 1));

        double m = meters(d);
        LinearLayout right = new LinearLayout(this);
        right.setOrientation(LinearLayout.VERTICAL);
        right.setGravity(Gravity.END);
        right.addView(label(distText(m), 20, m < 3 ? ORANGE : WHITE, true));
        right.addView(label(zone(m), 12, DIM, false));
        r.addView(right);

        Button ig = btn("무시", NAVY, DIM);
        ig.setTextSize(13);
        LinearLayout.LayoutParams ilp = new LinearLayout.LayoutParams(-2, dp(40));
        ilp.leftMargin = dp(10);
        ig.setLayoutParams(ilp);
        ig.setOnClickListener(v -> ignore(d));
        r.addView(ig);

        r.setOnClickListener(v -> openFinder(d.key));
        return r;
    }

    void openFinder(String key) {
        findKey = key;
        acked.add(key);
        main.setVisibility(View.GONE);
        findView.setVisibility(View.VISIBLE);
        stopFlash();
        renderFinder(System.currentTimeMillis());
    }

    void closeFinder() {
        findKey = null;
        findView.setVisibility(View.GONE);
        main.setVisibility(View.VISIBLE);
    }

    void renderFinder(long now) {
        Dev d = devs.get(findKey);
        if (d == null) { closeFinder(); return; }
        findName.setText(d.name);
        long ago = (now - d.lastSeen) / 1000;
        double m = meters(d);
        if (ago > 15) {
            findDist.setText("신호 없음");
            findDist.setTextColor(DIM);
            findTrend.setText(ago + "초째 안 보입니다. 핫스팟·블루투스를 껐을 수 있습니다.");
            findTrend.setTextColor(DIM);
        } else {
            findDist.setText(distText(m));
            findDist.setTextColor(m < 1.5 ? ORANGE : m < 4 ? LIME : WHITE);
            double tr = trend(d);
            if (tr >= 2.5) { findTrend.setText("▲ 가까워지는 중"); findTrend.setTextColor(LIME); }
            else if (tr <= -2.5) { findTrend.setText("▼ 멀어지는 중"); findTrend.setTextColor(ORANGE); }
            else { findTrend.setText("● 그대로"); findTrend.setTextColor(WHITE); }
        }
        findRaw.setText(zone(m) + " · 신호 " + Math.round(d.ema) + " dBm · " + d.kind
                + "\n태블릿을 들고 천천히 걸어 보세요. 숫자보다 ▲▼ 방향을 믿는 게 정확합니다."
                + (d.wifi ? "\n와이파이는 4초마다 갱신되어 조금 늦게 반응합니다." : ""));
        // 막대: -95(0%) ~ -35(100%)
        double pct = Math.max(0.03, Math.min(1, (d.ema + 95) / 60.0));
        View parent = (View) findBarFill.getParent();
        int w = parent.getWidth();
        if (w > 0) {
            ViewGroup.LayoutParams lp = findBarFill.getLayoutParams();
            lp.width = (int) (w * pct);
            findBarFill.setLayoutParams(lp);
        }
        findBarFill.setBackground(round(m < 1.5 ? ORANGE : LIME, 12));
        // 소리 켬이면 가까울수록 빠르게 삑
        if (sound && tone != null && ago <= 15) {
            long gap = (long) Math.max(250, Math.min(2000, m * 250));
            if (now - lastBeep >= gap) { tone.startTone(ToneGenerator.TONE_PROP_BEEP, 80); lastBeep = now; }
        }
    }

    // ───────────────────────── 화면 만들기 ─────────────────────────

    void buildUi() {
        root = new FrameLayout(this);
        root.setBackgroundColor(NAVY);
        root.setOnApplyWindowInsetsListener((v, ins) -> {
            v.setPadding(ins.getSystemWindowInsetLeft(), ins.getSystemWindowInsetTop(),
                    ins.getSystemWindowInsetRight(), ins.getSystemWindowInsetBottom());
            return ins.consumeSystemWindowInsets();
        });

        // ── 메인 ──
        main = new LinearLayout(this);
        main.setOrientation(LinearLayout.VERTICAL);
        int g = dp(18);
        main.setPadding(g, g, g, g);

        LinearLayout top = new LinearLayout(this);
        top.setGravity(Gravity.CENTER_VERTICAL);
        top.addView(label("시험감독 레이더", 22, WHITE, true));
        TextView beta = label("베타", 12, NAVY, true);
        beta.setBackground(round(LIME, 8));
        beta.setPadding(dp(8), dp(2), dp(8), dp(2));
        LinearLayout.LayoutParams blp = new LinearLayout.LayoutParams(-2, -2);
        blp.leftMargin = dp(8);
        top.addView(beta, blp);
        top.addView(new View(this), new LinearLayout.LayoutParams(0, 1, 1));
        TextView help = label("사용법", 14, SKY, true);
        help.setPadding(dp(10), dp(8), dp(4), dp(8));
        help.setOnClickListener(v -> showHelp());
        top.addView(help);
        main.addView(top);

        statusCard = new LinearLayout(this);
        statusCard.setOrientation(LinearLayout.VERTICAL);
        statusCard.setPadding(dp(20), dp(18), dp(20), dp(18));
        statusCard.setBackground(round(NAVY2, 18));
        status = label("준비", 30, WHITE, true);
        statusSub = label("", 15, WHITE, false);
        statusSub.setPadding(0, dp(4), 0, 0);
        statusCard.addView(status);
        statusCard.addView(statusSub);
        btnAck = btn("경고 확인 (깜빡임 끄기)", WHITE, NAVY);
        LinearLayout.LayoutParams alp = new LinearLayout.LayoutParams(-1, dp(48));
        alp.topMargin = dp(12);
        btnAck.setLayoutParams(alp);
        btnAck.setOnClickListener(v -> ackAll());
        statusCard.addView(btnAck);
        LinearLayout.LayoutParams sclp = new LinearLayout.LayoutParams(-1, -2);
        sclp.topMargin = dp(14);
        main.addView(statusCard, sclp);

        LinearLayout r1 = hrow(dp(12));
        btnBase = btn("① 기준 잡기 (30초)", NAVY2, WHITE);
        btnBase.setOnClickListener(v -> startBaseline());
        btnWatch = btn("② 감시 시작", LIME, NAVY);
        btnWatch.setOnClickListener(v -> toggleWatch());
        r1.addView(btnBase, weighted(dp(56), 0));
        r1.addView(btnWatch, weighted(dp(56), dp(10)));
        main.addView(r1);

        LinearLayout r2 = hrow(dp(10));
        btnRange = btn("", NAVY2, WHITE);
        btnRange.setTextSize(14);
        btnRange.setOnClickListener(v -> { wide = !wide; prefs.edit().putBoolean("wide", wide).apply(); });
        btnSound = btn("", NAVY2, WHITE);
        btnSound.setTextSize(14);
        btnSound.setOnClickListener(v -> { sound = !sound; prefs.edit().putBoolean("sound", sound).apply(); });
        r2.addView(btnRange, weighted(dp(46), 0));
        r2.addView(btnSound, weighted(dp(46), dp(10)));
        main.addView(r2);

        tip = label("", 14, NAVY, false);
        tip.setBackground(round(LIME, 12));
        tip.setPadding(dp(14), dp(10), dp(14), dp(10));
        LinearLayout.LayoutParams tlp = new LinearLayout.LayoutParams(-1, -2);
        tlp.topMargin = dp(12);
        main.addView(tip, tlp);

        TextView lh = label("새로 나타난 기기", 16, WHITE, true);
        lh.setPadding(0, dp(18), 0, dp(8));
        main.addView(lh);
        ScrollView sv = new ScrollView(this);
        list = new LinearLayout(this);
        list.setOrientation(LinearLayout.VERTICAL);
        sv.addView(list);
        main.addView(sv, new LinearLayout.LayoutParams(-1, 0, 1));

        baseInfo = label("", 13, DIM, false);
        baseInfo.setPadding(0, dp(8), 0, 0);
        main.addView(baseInfo);
        root.addView(main, new FrameLayout.LayoutParams(-1, -1));

        // ── 찾기 화면 ──
        findView = new LinearLayout(this);
        findView.setOrientation(LinearLayout.VERTICAL);
        findView.setPadding(g, g, g, g);
        findView.setVisibility(View.GONE);
        Button back = btn("← 목록으로", NAVY2, WHITE);
        back.setOnClickListener(v -> closeFinder());
        findView.addView(back, new LinearLayout.LayoutParams(-2, dp(48)));
        findName = label("", 24, WHITE, true);
        findName.setPadding(0, dp(24), 0, 0);
        findName.setGravity(Gravity.CENTER);
        findView.addView(findName, new LinearLayout.LayoutParams(-1, -2));
        findDist = label("", 96, LIME, true);
        findDist.setGravity(Gravity.CENTER);
        findView.addView(findDist, new LinearLayout.LayoutParams(-1, -2));
        findTrend = label("", 28, WHITE, true);
        findTrend.setGravity(Gravity.CENTER);
        findView.addView(findTrend, new LinearLayout.LayoutParams(-1, -2));
        FrameLayout bar = new FrameLayout(this);
        bar.setBackground(round(NAVY2, 12));
        LinearLayout.LayoutParams barLp = new LinearLayout.LayoutParams(-1, dp(28));
        barLp.topMargin = dp(28);
        findBarFill = new View(this);
        bar.addView(findBarFill, new FrameLayout.LayoutParams(0, -1));
        findView.addView(bar, barLp);
        LinearLayout ends = new LinearLayout(this);
        TextView far = label("멀다", 13, DIM, false);
        TextView near = label("가깝다", 13, DIM, false);
        ends.addView(far, new LinearLayout.LayoutParams(0, -2, 1));
        near.setGravity(Gravity.END);
        ends.addView(near, new LinearLayout.LayoutParams(0, -2, 1));
        findView.addView(ends);
        findRaw = label("", 15, WHITE, false);
        findRaw.setPadding(0, dp(18), 0, 0);
        findRaw.setGravity(Gravity.CENTER);
        findView.addView(findRaw, new LinearLayout.LayoutParams(-1, -2));
        View sp = new View(this);
        findView.addView(sp, new LinearLayout.LayoutParams(1, 0, 1));
        Button ign = btn("이 기기는 괜찮음 (무시하기)", NAVY2, WHITE);
        ign.setOnClickListener(v -> { Dev d = devs.get(findKey); if (d != null) ignore(d); });
        findView.addView(ign, new LinearLayout.LayoutParams(-1, dp(52)));
        root.addView(findView, new FrameLayout.LayoutParams(-1, -1));

        setContentView(root);
    }

    void showHelp() {
        new AlertDialog.Builder(this)
                .setTitle("사용법")
                .setMessage("① 학생 입실 전, 빈 교실에서 '기준 잡기'를 누릅니다(30초). 학교 공유기, 옆 반 기기, 감독관 폰이 '원래 있던 것'으로 저장됩니다.\n\n"
                        + "② 끝나면 저절로 감시가 시작됩니다. 새 신호가 가까이 나타나면 화면이 깜빡이고 진동합니다.\n\n"
                        + "③ 목록에서 항목을 누르면 거리 화면이 나옵니다. 태블릿을 들고 걸으며 ▲(가까워짐)가 뜨는 쪽으로 가세요.\n\n"
                        + "④ 괜찮은 기기면 '무시'를 누르면 다시 경고하지 않습니다.\n\n"
                        + "알아 둘 점\n"
                        + "· 잡히는 것: 핫스팟을 켠 폰, 블루투스를 켠 폰·이어폰·스마트워치\n"
                        + "· 안 잡히는 것: 비행기 모드·전원을 끈 폰, 핫스팟 없이 LTE만 쓰는 폰\n"
                        + "· 거리는 대략입니다. 몸·책상에 가리면 2~3배 차이 납니다.\n"
                        + "· 블루투스 주소를 바꾸는 폰 때문에 가끔 가짜 경고가 생길 수 있습니다. 같은 제조사 기기가 방금 사라졌으면 자동으로 걸러 냅니다.\n"
                        + "· 화면을 켜 둔 채로 앱을 띄워 두세요. 다른 앱으로 가면 검색이 멈춥니다.\n"
                        + "· 신호는 이 태블릿 안에서만 다루며 어디에도 보내지 않습니다.")
                .setPositiveButton("닫기", null).show();
    }

    // ───────────────────────── 작은 도우미 ─────────────────────────

    int dp(int v) { return (int) TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, v, getResources().getDisplayMetrics()); }

    TextView label(String s, int sp, int color, boolean bold) {
        TextView t = new TextView(this);
        t.setText(s);
        t.setTextSize(sp);
        t.setTextColor(color);
        if (bold) t.setTypeface(Typeface.DEFAULT_BOLD);
        return t;
    }

    Button btn(String s, int bg, int fg) {
        Button b = new Button(this);
        b.setText(s);
        b.setAllCaps(false);
        b.setTextSize(16);
        b.setStateListAnimator(null);
        styleBtn(b, bg, fg);
        return b;
    }

    void styleBtn(Button b, int bg, int fg) {
        b.setBackground(round(bg, 14));
        b.setTextColor(fg);
        b.setTypeface(Typeface.DEFAULT_BOLD);
    }

    LinearLayout hrow(int topMargin) {
        LinearLayout r = new LinearLayout(this);
        r.setOrientation(LinearLayout.HORIZONTAL);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(-1, -2);
        lp.topMargin = topMargin;
        r.setLayoutParams(lp);
        return r;
    }

    LinearLayout.LayoutParams weighted(int height, int leftMargin) {
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(0, height, 1);
        lp.leftMargin = leftMargin;
        return lp;
    }

    static GradientDrawable round(int color, int radiusDp) { return round(color, radiusDp, 0); }

    static GradientDrawable round(int color, int radiusDp, int stroke) {
        GradientDrawable d = new GradientDrawable();
        d.setColor(color);
        float px = radiusDp * android.content.res.Resources.getSystem().getDisplayMetrics().density;
        d.setCornerRadius(px);
        if (stroke != 0) d.setStroke((int) (2 * android.content.res.Resources.getSystem().getDisplayMetrics().density), stroke);
        return d;
    }
}
