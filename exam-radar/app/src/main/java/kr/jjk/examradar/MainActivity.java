package kr.jjk.examradar;

import android.Manifest;
import android.animation.ArgbEvaluator;
import android.animation.ValueAnimator;
import android.app.Activity;
import android.app.AlertDialog;
import android.app.PendingIntent;
import android.content.pm.PackageInstaller;
import android.widget.Toast;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
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
 * 1) 기준 잡기: 빈 교실에서 1분 동안 보이는 와이파이·블루투스 신호를 '원래 있던 것'으로 저장
 * 2) 감시: 기준에 없던 신호가 가까이 나타나면 화면이 깜빡이고 진동
 * 3) 찾기: 항목을 누르면 대략 거리와 가까워짐/멀어짐을 실시간으로 표시
 */
public class MainActivity extends Activity {

    // 색: 디자인.md '대비 강한 조합' + 경고는 쨍한 오렌지
    static final int NAVY = 0xFF111827, NAVY2 = 0xFF1F2A44, WHITE = 0xFFF3F4F6, LIME = 0xFFA3E635,
            ORANGE = 0xFFFF4C02, SKY = 0xFF60A5FA, DIM = 0xFF8FA3C7;

    static final int MODE_IDLE = 0, MODE_BASELINE = 1, MODE_WATCH = 2;
    static final int BASELINE_SECONDS = 60;
    static final long GONE_MS = 45_000;      // 이만큼 안 보이면 사라진 것으로
    static final int REQ_PERM = 7;

    static class Dev {
        String key, name, kind;   // kind: 기기 종류 설명
        boolean wifi, phoneLike, named, direct;
        boolean gone;             // 와이파이: 최근 검색에 두 번 연속 안 나옴(꺼짐)
        int miss;
        int lastLoc;              // 마지막으로 잡혔을 때 감독관이 서 있던 곳(순회)
        String typeShort, typeLong; // 블루투스 기기 종류 짐작(휴대폰·이어폰·시계·TV 등)
        int rank = 1;             // 3 휴대폰 · 2 개인 기기 · 1 모름 · 0 TV·비콘·PC 등
        int manu = -1;            // 블루투스 제조사 번호
        int rssi;
        int txPower = 127;        // 블루투스가 알려 주는 송신 세기(127 = 모름)
        double ema = Double.NaN;
        long firstSeen, lastSeen;
        int hits;
        final ArrayList<double[]> hist = new ArrayList<>(); // [시각, ema]
        final ArrayList<long[]> raw = new ArrayList<>();    // [시각, 원본 세기] 최근 5초
        final HashMap<Integer, Double> roomPeak = new HashMap<>(); // 순회: 교실별 가장 센 세기
        final HashMap<Integer, Double> roomMed = new HashMap<>();  // 순회: 멈춰서 잰 교실별 중앙값
        final HashMap<Integer, Integer> roomCnt = new HashMap<>(); // 그 곳을 몇 번 쟀나
        final HashSet<Integer> roomShort = new HashSet<>();        // 표본 부족으로 판정 보류한 곳
    }

    final Map<String, Dev> devs = new HashMap<>();
    final Set<String> baseline = new HashSet<>();
    final Set<String> acked = new HashSet<>();   // 경고 확인을 누른 새 기기
    int mode = MODE_IDLE;
    long baselineEnd, baselineStart;
    boolean wide = false, sound = false;
    String findKey = null;
    boolean patrol = false;   // 복도 순회 모드
    int room = 1;             // 순회: 지금 서 있는 반 번호
    int grade = 1;            // 순회: 학년 (위치 코드 = 학년×100 + 반)

    int loc() { return grade * 100 + room; }

    // 학년·반이 아닌 곳(교무실·과학실 등): 학년 자리에 9, 위치 코드 = 900 + 순번
    static final ArrayList<String> places = new ArrayList<>();

    static String roomName(int code) {
        if (code >= 900) { int i = code - 901; return i >= 0 && i < places.size() ? places.get(i) : "장소 " + (i + 1); }
        return code >= 100 ? (code / 100) + "학년 " + (code % 100) + "반" : code + "반";
    }

    void loadPlaces() {
        places.clear();
        try {
            org.json.JSONArray a = new org.json.JSONArray(prefs.getString("places", "[]"));
            for (int i = 0; i < a.length(); i++) places.add(a.getString(i));
        } catch (Exception ignored) { }
    }

    void savePlaces() { prefs.edit().putString("places", new org.json.JSONArray(places).toString()).apply(); }

    int placeCode(String name) {
        name = name.trim();
        int i = places.indexOf(name);
        if (i < 0) { places.add(name); savePlaces(); i = places.size() - 1; }
        return 901 + i;
    }

    void choose(int code) {
        grade = code / 100; room = code % 100;
        prefs.edit().putInt("grade", grade).putInt("room", room).apply();
        render(System.currentTimeMillis());
    }

    /** 학년 버튼 1·2·3 + '기타 장소' */
    void addGradeButtons(LinearLayout row, Button[] arr) {
        for (int gi = 0; gi < 4; gi++) {
            final int gg = gi + 1;
            arr[gi] = btn(gi < 3 ? gg + "학년" : "기타 장소", NAVY, WHITE);
            arr[gi].setTextSize(gi < 3 ? 16 : 14);
            arr[gi].setOnClickListener(v -> { if (gg <= 3) choose(gg * 100 + 1); else pickPlace(); });
            row.addView(arr[gi], weighted(dp(46), gi == 0 ? 0 : dp(6)));
        }
    }

    void styleGradeButtons(Button[] arr) {
        for (int i = 0; i < 4; i++) {
            boolean on = i < 3 ? grade == i + 1 : grade == 9;
            styleBtn(arr[i], on ? WHITE : NAVY, on ? NAVY : WHITE);
        }
    }

    void pickPlace() {
        String[] items = new String[places.size() + 1];
        for (int i = 0; i < places.size(); i++) items[i] = places.get(i);
        items[places.size()] = "+ 새 장소 추가 (교무실, 과학실 등)";
        new AlertDialog.Builder(this).setTitle("어느 곳인가요?")
                .setItems(items, (d, w) -> { if (w < places.size()) choose(901 + w); else askPlaceName(code -> choose(code)); })
                .show();
    }

    interface CodeCb { void on(int code); }

    /** 이름을 받아 위치 코드로: "2학년 3반"·"2-3"이면 반, 아니면 장소 */
    void askPlaceName(CodeCb cb) {
        android.widget.EditText et = new android.widget.EditText(this);
        et.setHint("예: 교무실, 2학년 3반, 2-3");
        et.setSingleLine(true);
        new AlertDialog.Builder(this).setTitle("곳 이름").setView(et)
                .setPositiveButton("확인", (d, w) -> {
                    String s = et.getText().toString().trim();
                    if (s.isEmpty()) return;
                    java.util.regex.Matcher m = java.util.regex.Pattern.compile("^([1-3])\\s*(?:학년|-)\\s*(\\d{1,2})\\s*반?$").matcher(s);
                    if (m.find()) cb.on(Integer.parseInt(m.group(1)) * 100 + Integer.parseInt(m.group(2)));
                    else cb.on(placeCode(s));
                })
                .setNegativeButton("취소", null).show();
    }

    /** 잘못된 곳에 저장된 기준을 맞는 곳으로 옮긴다(이미 있으면 합친다) */
    void moveRoom(int from, int to) {
        if (from == to) return;
        RoomBase src = rooms.remove(from);
        if (src == null) return;
        RoomBase dst = roomOf(to);
        dst.keys.addAll(src.keys);
        dst.fp.putAll(src.fp);
        dst.t = Math.max(dst.t, src.t);
        rebuildBaseline();
        saveRooms();
        toast(roomLabel(from) + " → " + roomLabel(to) + " 로 옮겼습니다.");
        render(System.currentTimeMillis());
    }
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
    Button btnBase, btnWatch, btnRange, btnSound, btnAck, btnPatrol, btnUpdate, btnClearAll;
    LinearLayout patrolBox;
    final Button[] gradeBtns = new Button[4];
    Button btnMeasure;
    TextView roomText, topText;
    Button tabWifi, tabBle, tabAll;
    int tab = 0; // 0 전체(두 칸으로 나눔), 1 와이파이만, 2 블루투스만

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
        patrol = prefs.getBoolean("patrol", true); // 주로 복도 순회로 쓴다(사용자)
        room = prefs.getInt("room", 1);
        grade = prefs.getInt("grade", 1);
        loadLog();
        loadPlaces();
        loadRooms();
        wifi = (WifiManager) getApplicationContext().getSystemService(Context.WIFI_SERVICE);
        vib = (Vibrator) getSystemService(Context.VIBRATOR_SERVICE);
        try { tone = new ToneGenerator(AudioManager.STREAM_MUSIC, 80); } catch (Exception ignored) { }
        buildUi();
        IntentFilter inf = new IntentFilter(ACTION_INSTALL);
        if (Build.VERSION.SDK_INT >= 33) registerReceiver(installRx, inf, Context.RECEIVER_NOT_EXPORTED);
        else registerReceiver(installRx, inf);
        askPermissions();
        checkUpdate(false);
        Intent in0 = getIntent();
        if (in0 != null && (Intent.ACTION_VIEW.equals(in0.getAction()) || Intent.ACTION_SEND.equals(in0.getAction()))) handleIncoming(in0);
        else if (rooms.isEmpty() && !prefs.getBoolean("askedImport", false)) {
            prefs.edit().putBoolean("askedImport", true).apply();
            new AlertDialog.Builder(this)
                    .setTitle("다른 태블릿에서 쓰던 자료가 있나요?")
                    .setMessage("쓰던 태블릿의 '기준' 탭 → '다른 태블릿으로 보내기'로 받은 파일이 있으면 바로 가져올 수 있습니다. 반별 기준·기록·설정이 한 번에 옮겨집니다.")
                    .setPositiveButton("가져오기", (d, w) -> pickImport())
                    .setNegativeButton("처음 쓰는 것", null).show();
        }
    }

    // ───────────────────────── 앱 안 업데이트 ─────────────────────────

    static final String UPD_BASE = "https://jaenamking1-collab.github.io/jjk/exam-radar/";
    static final String ACTION_INSTALL = "kr.jjk.examradar.INSTALL_RESULT";
    int remoteCode = 0;
    String remoteName = "";

    @SuppressWarnings("deprecation")
    int myCode() {
        try { return getPackageManager().getPackageInfo(getPackageName(), 0).versionCode; } catch (Exception e) { return 0; }
    }

    String myName() {
        try { return getPackageManager().getPackageInfo(getPackageName(), 0).versionName; } catch (Exception e) { return "?"; }
    }

    static HttpURLConnection open(String u) throws Exception {
        HttpURLConnection c = (HttpURLConnection) new URL(u).openConnection();
        c.setConnectTimeout(8000);
        c.setReadTimeout(20000);
        c.setUseCaches(false);
        return c;
    }

    /** 인터넷이 있으면 새 버전이 있는지 본다. 시험장에서 인터넷이 없어도 앱은 그대로 돈다. */
    void checkUpdate(boolean manual) {
        new Thread(() -> {
            try {
                HttpURLConnection c = open(UPD_BASE + "version.json?t=" + System.currentTimeMillis());
                StringBuilder sb = new StringBuilder();
                try (InputStream in = c.getInputStream()) {
                    byte[] b = new byte[4096]; int r;
                    while ((r = in.read(b)) > 0) sb.append(new String(b, 0, r, "UTF-8"));
                }
                org.json.JSONObject j = new org.json.JSONObject(sb.toString());
                int code = j.getInt("versionCode");
                String name = j.optString("versionName", "");
                runOnUiThread(() -> {
                    remoteCode = code; remoteName = name;
                    if (code > myCode()) {
                        btnUpdate.setVisibility(View.VISIBLE);
                        btnUpdate.setEnabled(true);
                        btnUpdate.setText("새 버전 " + name + " 업데이트");
                    } else {
                        btnUpdate.setVisibility(View.GONE);
                        if (manual) toast("최신 버전입니다 (" + myName() + ")");
                    }
                });
            } catch (Exception e) {
                if (manual) runOnUiThread(() -> toast("인터넷에 연결되어 있지 않아 확인하지 못했습니다."));
            }
        }).start();
    }

    void doUpdate() {
        if (Build.VERSION.SDK_INT >= 26 && !getPackageManager().canRequestPackageInstalls()) {
            new AlertDialog.Builder(this)
                    .setTitle("한 번만 허용해 주세요")
                    .setMessage("이 앱이 스스로 업데이트하려면 '이 출처 허용'을 켜야 합니다.\n다음 화면에서 켠 뒤 뒤로 돌아와 업데이트를 다시 누르세요.")
                    .setPositiveButton("허용하러 가기", (d, w) -> startActivity(new Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                            android.net.Uri.parse("package:" + getPackageName()))))
                    .setNegativeButton("나중에", null).show();
            return;
        }
        btnUpdate.setEnabled(false);
        btnUpdate.setText("받는 중…");
        new Thread(() -> {
            try {
                PackageInstaller pi = getPackageManager().getPackageInstaller();
                int id = pi.createSession(new PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL));
                try (PackageInstaller.Session s = pi.openSession(id)) {
                    HttpURLConnection c = open(UPD_BASE + "exam-radar-beta.apk?t=" + System.currentTimeMillis());
                    long len = c.getContentLengthLong();
                    try (InputStream in = c.getInputStream(); OutputStream out = s.openWrite("app.apk", 0, len > 0 ? len : -1)) {
                        byte[] b = new byte[65536]; int r; long tot = 0;
                        while ((r = in.read(b)) > 0) {
                            out.write(b, 0, r); tot += r;
                            if (len > 0) { final int pct = (int) (tot * 100 / len); runOnUiThread(() -> btnUpdate.setText("받는 중 " + pct + "%")); }
                        }
                        s.fsync(out);
                    }
                    Intent i = new Intent(ACTION_INSTALL).setPackage(getPackageName());
                    int fl = PendingIntent.FLAG_UPDATE_CURRENT | (Build.VERSION.SDK_INT >= 31 ? PendingIntent.FLAG_MUTABLE : 0);
                    s.commit(PendingIntent.getBroadcast(this, 1, i, fl).getIntentSender());
                }
                runOnUiThread(() -> btnUpdate.setText("설치 확인 창을 기다리는 중…"));
            } catch (Exception e) {
                runOnUiThread(() -> {
                    btnUpdate.setEnabled(true);
                    btnUpdate.setText("업데이트 다시 시도");
                    toast("받지 못했습니다. 와이파이 연결을 확인해 주세요.");
                });
            }
        }).start();
    }

    final BroadcastReceiver installRx = new BroadcastReceiver() {
        @Override public void onReceive(Context c, Intent i) {
            int st = i.getIntExtra(PackageInstaller.EXTRA_STATUS, -99);
            if (st == PackageInstaller.STATUS_PENDING_USER_ACTION) {
                @SuppressWarnings("deprecation") Intent confirm = i.getParcelableExtra(Intent.EXTRA_INTENT);
                if (confirm != null) { confirm.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK); startActivity(confirm); }
            } else if (st != PackageInstaller.STATUS_SUCCESS) {
                btnUpdate.setEnabled(true);
                btnUpdate.setText("업데이트 다시 시도");
                toast("설치되지 않았습니다. 다시 눌러 주세요.");
            }
        }
    };

    void toast(String s) { Toast.makeText(this, s, Toast.LENGTH_LONG).show(); }

    @Override
    protected void onResume() {
        super.onResume();
        if (hasPerms()) startScanning();
        h.removeCallbacks(tick);
        h.post(tick);
        // 앱으로 돌아올 때도 새 버전 확인(10분에 한 번까지)
        long t = System.currentTimeMillis();
        if (t - lastUpdCheck > 10 * 60_000L) { lastUpdCheck = t; checkUpdate(false); }
    }

    long lastUpdCheck = System.currentTimeMillis();

    @Override
    protected void onPause() {
        super.onPause();
        // 감시 중이면 화면이 켜져 있는 동안 계속 돈다. 앱을 벗어나면 검색을 멈춘다.
        stopScanning();
        h.removeCallbacks(tick);
        saveLog();
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        if (tone != null) tone.release();
        try { unregisterReceiver(installRx); } catch (Exception ignored) { }
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
        long newest = 0;
        for (android.net.wifi.ScanResult r : rs) newest = Math.max(newest, now - (nowUs - r.timestamp) / 1000L);
        // 새 검색 결과가 왔는데 거기에 두 번 연속 안 나온 와이파이는 꺼진 것으로 본다.
        // (한 번만 빠진 건 일부 채널만 훑은 검색일 수 있어서 한 번 더 본다)
        if (newest > lastBatch + 1000) {
            lastBatch = newest;
            for (Dev d : devs.values()) {
                if (!d.wifi) continue;
                if (newest - d.lastSeen > 5000) { d.miss++; if (d.miss >= 2) d.gone = true; }
            }
        }
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
            // 와이파이 다이렉트(DIRECT-…)는 프린터·TV·화면 공유기가 대부분. 이름에 휴대폰 단서가 있을 때만 휴대폰으로 본다.
            d.direct = ssid.startsWith("DIRECT-");
            d.phoneLike = d.direct ? ssid.toLowerCase(Locale.ROOT).matches(".*(galaxy|iphone|phone|sm-|redmi|pixel|android).*")
                    : looksLikePhone(ssid, r);
            d.kind = (d.phoneLike ? "휴대폰 핫스팟 의심 · " : d.direct ? "와이파이 다이렉트(프린터·TV·화면 공유 등) · " : "와이파이 · ") + (r.frequency >= 5900 ? "6GHz" : r.frequency >= 4900 ? "5GHz" : "2.4GHz");
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
        @Override public void onScanResult(int type, ScanResult r) { lastBleResult = System.currentTimeMillis(); bleFail = 0; onBle(r); }
        @Override public void onBatchScanResults(List<ScanResult> rs) { lastBleResult = System.currentTimeMillis(); for (ScanResult r : rs) onBle(r); }
        @Override public void onScanFailed(int code) { bleFail = code; ble = null; }
    };

    // 검색이 살아 있는지: '아무것도 안 잡힘'과 '검색이 멈춤'을 구별하려고
    long lastBleResult = 0, lastWifiFresh = 0;
    int bleFail = 0;

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
        if (nm != null && !nm.trim().isEmpty()) d.named = true;
        String maker = maker(manu);
        d.name = (nm != null && !nm.trim().isEmpty()) ? nm.trim() : (maker != null ? maker + " 기기" : "이름 없는 기기");
        classifyBle(d, rec, nm);
        d.phoneLike = d.rank == 3;
        d.kind = "블루투스 · " + (d.typeLong != null ? d.typeLong : maker != null ? maker + " 기기 (종류 모름)" : "종류 모름");
        int tx = r.getTxPower();
        if (tx != ScanResult.TX_POWER_NOT_PRESENT) d.txPower = tx;
        else if (rec != null && rec.getTxPowerLevel() != Integer.MIN_VALUE) d.txPower = rec.getTxPowerLevel();
        feed(d, r.getRssi(), now);
        if (fresh && mode == MODE_WATCH) absorbRotation(d);
    }

    // ───────────────────────── 블루투스 기기 종류 짐작 ─────────────────────────
    // 우선순위(rank): 3 휴대폰 · 2 이어폰/시계/종류 있는 개인 기기 · 1 모름 · 0 TV·비콘·노트북 등(시험과 거리 먼 것)

    static void setType(Dev d, String shortName, String longName, int rank) {
        // 더 확실한(우선순위 높은) 단서가 이미 있으면 덮어쓰지 않는다
        if (d.typeShort != null && d.rank > rank) return;
        d.typeShort = shortName; d.typeLong = longName; d.rank = rank;
    }

    static int uuid16(android.os.ParcelUuid u) {
        String s = u.getUuid().toString();
        if (!s.endsWith("-0000-1000-8000-00805f9b34fb")) return -1;
        try { return Integer.parseInt(s.substring(4, 8), 16); } catch (Exception e) { return -1; }
    }

    void classifyBle(Dev d, ScanRecord rec, String nm) {
        if (rec == null) return;
        // 1) 이름
        if (nm != null) {
            String n = nm.toLowerCase(Locale.ROOT);
            if (n.matches(".*(buds|airpods|pods|earbud|earphone|headset|헤드셋|이어폰|wf-|wh-|tws|qcy|beats).*")) setType(d, "이어폰", "이어폰·헤드셋 (이름)", 2);
            else if (n.matches(".*(watch|band|fit|워치|밴드|garmin|amazfit).*")) setType(d, "시계·밴드", "스마트워치·밴드 (이름)", 2);
            else if (n.matches(".*(iphone|galaxy [asmz]|galaxy note|phone|폰|redmi|pixel|sm-[agsnf]).*")) setType(d, "휴대폰", "휴대폰 (이름)", 3);
            else if (n.matches(".*(ipad|tab|macbook|laptop|desktop|-pc|노트북|book).*")) setType(d, "태블릿·PC", "태블릿·노트북 (이름)", 0);
            else if (n.matches(".*(tv|\\[tv\\]|speaker|soundbar|스피커|jbl|projector|프로젝터|printer|프린터|chromecast).*")) setType(d, "TV·스피커", "TV·스피커·프린터 등 (이름)", 0);
        }
        // 2) 기기 겉모습 번호(Appearance) — 광고 원본에서 직접 읽는다
        byte[] raw = rec.getBytes();
        if (raw != null) {
            int i = 0;
            while (i < raw.length - 1) {
                int len = raw[i] & 0xff;
                if (len == 0 || i + len >= raw.length) break;
                int type = raw[i + 1] & 0xff;
                if (type == 0x19 && len >= 3) {
                    int ap = (raw[i + 2] & 0xff) | ((raw[i + 3] & 0xff) << 8);
                    int cat = ap >> 6;
                    if (cat == 1) setType(d, "휴대폰", "휴대폰 (기기 표시)", 3);
                    else if (cat == 3) setType(d, "시계·밴드", "시계 (기기 표시)", 2);
                    else if (cat == 33) setType(d, "이어폰", "이어폰·헤드셋 (기기 표시)", 2);
                    else if (cat == 2) setType(d, "태블릿·PC", "컴퓨터 (기기 표시)", 0);
                    else if (cat == 5 || cat == 10) setType(d, "TV·스피커", "화면·미디어 기기 (기기 표시)", 0);
                    else if (cat == 8 || cat == 9) setType(d, "태그", "분실방지 태그 (기기 표시)", 1);
                    else if (cat == 15) setType(d, "입력기기", "키보드·마우스 등 (기기 표시)", 0);
                }
                i += len + 1;
            }
        }
        // 3) 애플: 신호 유형(첫 바이트)으로 종류가 꽤 갈린다(공개 분석 자료 기준, 추정)
        byte[] ap = rec.getManufacturerSpecificData(0x004C);
        if (ap != null && ap.length >= 1) {
            int i = 0;
            while (i < ap.length) {
                int t = ap[i] & 0xff;
                if (t == 0x10 || t == 0x05 || t == 0x0F) setType(d, "휴대폰", "애플 휴대폰·패드 (켜져 있음, 추정)", 3);
                else if (t == 0x07) setType(d, "이어폰", "에어팟 등 애플 이어폰 (추정)", 2);
                else if (t == 0x0C) setType(d, "휴대폰", "애플 기기 (휴대폰·맥 사용 중, 추정)", 3);
                else if (t == 0x12) setType(d, "태그", "애플 '나의 찾기' 신호 (에어태그 또는 주인과 떨어진 애플 기기)", 1);
                else if (t == 0x09) setType(d, "TV·스피커", "애플 TV·스피커 (추정)", 0);
                if (i + 1 >= ap.length) break;
                i += 2 + (ap[i + 1] & 0xff);
            }
        }
        // 4) 서비스 번호
        List<android.os.ParcelUuid> us = new ArrayList<>();
        if (rec.getServiceUuids() != null) us.addAll(rec.getServiceUuids());
        if (rec.getServiceData() != null) us.addAll(rec.getServiceData().keySet());
        for (android.os.ParcelUuid u : us) {
            switch (uuid16(u)) {
                case 0xFE2C: setType(d, "이어폰", "이어폰 등 (구글 빠른 연결 신호)", 2); break;
                case 0xFD5A: case 0xFD69: setType(d, "삼성 기기", "삼성 휴대폰·이어폰·워치 중 하나 (내 디바이스 찾기 신호)", 2); break;
                case 0xFEF3: case 0xFE9F: case 0xFCF1: setType(d, "안드로이드", "안드로이드 기기 추정 (구글 서비스 신호)", 2); break;
                case 0xFD6F: setType(d, "휴대폰", "휴대폰 (접촉 확인 신호)", 3); break;
                case 0xFEED: case 0xFEEC: setType(d, "태그", "분실방지 태그(Tile 등)", 1); break;
                case 0xFE95: setType(d, "시계·밴드", "샤오미 기기(밴드 등) 추정", 2); break;
                case 0x180D: setType(d, "시계·밴드", "심박 측정 기기(밴드·시계)", 2); break;
                case 0xFEAA: setType(d, "비콘", "비콘(Eddystone)", 0); break;
                default: break;
            }
        }
        // 5) 제조사만 있을 때의 대략
        int m = d.manu;
        if (d.typeShort == null) {
            if (m == 0x0006) setType(d, "태블릿·PC", "윈도우 PC·마우스 등 (마이크로소프트 신호)", 0);
            else if (m == 0x00C4) setType(d, "TV·스피커", "LG 기기 (TV 등 추정)", 0);
            else if (m == 0x0075) setType(d, "삼성 기기", "삼성 기기 (휴대폰·이어폰·워치·TV 중, 종류 모름)", 1);
            else if (m == 0x004C) setType(d, "애플 기기", "애플 기기 (종류 모름)", 1);
            else if (m == 0x0087) setType(d, "시계·밴드", "가민 시계 추정", 2);
            else if (m == 0x0157 || m == 0x038F) setType(d, "시계·밴드", "샤오미 기기(밴드 등) 추정", 2);
        }
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
            if (!baseline.contains(o.key) && !sessionIgnored.contains(o.key)) continue;
            // 같은 종류여야 한다(휴대폰↔휴대폰). 종류를 모르면 그대로 둔다.
            if (o.typeShort != null && nd.typeShort != null && !o.typeShort.equals(nd.typeShort)) continue;
            // 진짜 주소 바뀜은 옛 주소가 끊기는 '바로 그때' 새 주소가 나온다: 1.5~8초 사이만 인정.
            // (블루투스를 껐다 켜면 보통 8초 넘게 걸려 새 신호로 경고된다. 2026-11-27 실험에서 2분 기준으로는 껐다 켠 갤럭시가 묻혔다)
            long gap = now - o.lastSeen;
            if (gap < 1_500 || gap > 8_000) continue;
            if (!Double.isNaN(o.ema) && Math.abs(o.ema - nd.rssi) <= 8) {
                // 같은 기기라는 보장이 없으므로 기준에 넣지 않고, 이번 감시 동안만 '확인 필요'로 낮춘다
                // 시작 때 정리한 기기가 주소를 바꾼 것이면 계속 빼 주고, 기준 기기면 '확인 필요'로만 낮춘다
                if (sessionIgnored.contains(o.key)) sessionIgnored.add(nd.key); else rotCand.add(nd.key);
                return;
            }
        }
    }

    void feed(Dev d, int rssi, long at) {
        d.rssi = rssi;
        // 찾는 중인 기기는 더 빨리 따라가게(반응 우선), 나머지는 차분하게(흔들림 줄이기)
        double a = 0.4;
        d.raw.add(new long[]{at, rssi});
        while (!d.raw.isEmpty() && at - d.raw.get(0)[0] > 5000) d.raw.remove(0);
        d.ema = Double.isNaN(d.ema) ? rssi : d.ema * (1 - a) + rssi * a;
        d.lastSeen = Math.max(d.lastSeen, at);
        d.hits++;
        d.gone = false;
        d.miss = 0;
        d.lastLoc = loc();
        d.hist.add(new double[]{at, d.ema});
        while (d.hist.size() > 40) d.hist.remove(0);
        if (mode == MODE_BASELINE) addBase(d);
        if (patrol && mode == MODE_WATCH) {
            Double p = d.roomPeak.get(loc());
            if (p == null || d.ema > p) d.roomPeak.put(loc(), d.ema);
        }
        if (dirScanning && d.key.equals(findKey)) {
            int b = ((int) (azimuth / 30)) % 12;
            binSum[b] += rssi; binN[b]++;
        }
        if (d.wifi) lastWifiFresh = Math.max(lastWifiFresh, at);
        // 재기 시작 전에 관측된 값(검색 실패 때 다시 오는 옛 결과 등)은 이 곳 값으로 세지 않는다
        if ((measuring && at >= measureStart) || (surveying && at >= surveyStart)) {
            ArrayList<Integer> l = mbuf.get(d.key);
            if (l == null) { l = new ArrayList<>(); mbuf.put(d.key, l); }
            l.add(rssi);
        }
    }

    // ───────────────────────── 순회: 멈춰서 재기 ─────────────────────────

    // 고정 시간 대신 '충분히 모이면' 끝낸다: 최소 5초, 최대 15초.
    // 블루투스는 1초에 여러 번 와서 금방 15개가 차고, 와이파이는 4초마다 1번이라 3번 모이려면 12초쯤 걸린다.
    static final int MEASURE_MS = 15000, MEASURE_MIN_MS = 5000;
    boolean measuring = false;
    long measureEnd = 0, measureStart = 0;

    boolean enoughSamples(long now) {
        for (Dev d : devs.values()) {
            if (!isAlert(d, now)) continue;
            ArrayList<Integer> l = mbuf.get(d.key);
            if (l == null || l.size() < (d.wifi ? 3 : 15)) return false;
        }
        return true;
    }
    final Map<String, ArrayList<Integer>> mbuf = new HashMap<>();

    void startMeasure() {
        if (mode != MODE_WATCH) { toast("먼저 기준을 잡고 감시를 시작해 주세요."); return; }
        if (measuring) return;
        mbuf.clear();
        measuring = true;
        measureStart = System.currentTimeMillis();
        measureEnd = measureStart + MEASURE_MS;
        kickWifiScan();
        render(System.currentTimeMillis());
    }

    /** 재는 동안 모은 값의 '가운데 값'(중앙값)을 그 교실 값으로 쓴다. 순간적으로 튀는 값에 덜 흔들린다. */
    void finishMeasure() {
        measuring = false;
        int n = 0;
        for (Map.Entry<String, ArrayList<Integer>> e : mbuf.entrySet()) {
            Dev d = devs.get(e.getKey());
            ArrayList<Integer> l = e.getValue();
            if (d == null || l.isEmpty()) continue;
            // 와이파이는 새 값 2개, 블루투스는 5개가 안 되면 이 곳 판정을 보류한다
            if (l.size() < (d.wifi ? 2 : 5)) { d.roomShort.add(loc()); continue; }
            d.roomShort.remove(loc());
            Collections.sort(l);
            double med = l.size() % 2 == 1 ? l.get(l.size() / 2) : (l.get(l.size() / 2 - 1) + l.get(l.size() / 2)) / 2.0;
            d.roomMed.put(loc(), med);
            Integer c = d.roomCnt.get(loc());
            d.roomCnt.put(loc(), c == null ? 1 : c + 1);
            n++;
        }
        mbuf.clear();
        buzz(new long[]{0, 80});
        if (sound && tone != null) tone.startTone(ToneGenerator.TONE_PROP_ACK, 150);
        toast(roomName(loc()) + " 앞 측정 끝 (신호 " + n + "개). 다음 반으로 넘어갑니다.");
        if (grade != 9) setRoom(room + 1); else render(System.currentTimeMillis());
    }

    /**
     * 순회 판정. 세기 차이는 몸 가림만으로도 10dB 넘게 흔들릴 수 있어 '확실'이라 하지 않는다.
     * '잰 곳 중' 상대 비교라는 점과, 기기가 잰 곳 밖(위층·복도)에 있을 수도 있다는 점을 드러낸다.
     */
    static String verdict(Dev d) {
        Map<Integer, Double> m = d.roomMed;
        if (m.isEmpty()) return d.roomShort.isEmpty() ? null : (d.wifi ? "와이파이 표본 부족 · 이 곳 판정 보류" : "표본 부족 · 이 곳 판정 보류");
        int b1 = -1, b2 = -1; double v1 = -999, v2 = -999;
        for (Map.Entry<Integer, Double> e : m.entrySet()) {
            double v = e.getValue();
            if (v > v1) { b2 = b1; v2 = v1; b1 = e.getKey(); v1 = v; }
            else if (v > v2) { b2 = e.getKey(); v2 = v; }
        }
        Integer cnt = d.roomCnt.get(b1);
        String again = cnt != null && cnt >= 2 ? " · " + cnt + "번 잼" : "";
        if (v1 < -85) return "잰 곳 모두 약함 · 판정 불가 (잰 곳 밖일 수 있음)";
        if (b2 < 0) return "잰 곳: " + roomName(b1) + "뿐 · 옆도 재야 비교됩니다" + again;
        double diff = v1 - v2;
        if (diff < 3) return "차이 근소 · 판정 불가 (" + roomName(b1) + "·" + roomName(b2) + " 같은 자세로 다시 재기)";
        String c = diff >= 6 ? "잰 곳 중 뚜렷하게 강함" : "잰 곳 중 조금 강함";
        return roomName(b1) + " 쪽 · " + c + " (" + roomName(b2) + "보다 " + Math.round(diff) + "dB)" + again;
    }

    // ───────────────────────── 판단 ─────────────────────────

    // 순회 모드는 복도에서 벽 너머 교실을 봐야 하니 항상 넓게
    int wifiLimit() { return (wide || patrol) ? -88 : -75; }
    int bleLimit() { return (wide || patrol) ? -85 : -72; }

    /** 순회: 이 기기 신호가 가장 셌던 교실 번호(없으면 -1) */
    static int bestRoom(Dev d) {
        int best = -1; double v = -999;
        for (Map.Entry<Integer, Double> e : d.roomMed.entrySet())
            if (e.getValue() > v) { v = e.getValue(); best = e.getKey(); }
        return best;
    }

    static String trendWord(Dev d) {
        double tr = trend(d);
        return tr >= 2.5 ? "▲ 가까워짐" : tr <= -2.5 ? "▼ 멀어짐" : "● 그대로";
    }

    boolean showUnknown = false; // '확인 필요' 칸 펼침
    boolean healthy = true;      // 검색이 살아 있나

    /** 이름도 제조사도 없는 블루투스 신호: 비콘·가전·주소를 바꾸는 기기 조각이 대부분이라 기본은 숨김 */
    static boolean noise(Dev d) { return !d.wifi && !d.named && maker(d.manu) == null && d.typeShort == null; }

    /** 이름 없는 신호라도 아주 가깝고(약 2m 안) 30초 넘게 머물면 책상 위 기기일 수 있어 경고한다 */
    static boolean closeAndStaying(Dev d, long now) { return d.ema >= -62 && now - d.firstSeen >= 30_000; }

    // ── 세 칸 분류: 새 신호(경고) / 확인 필요(표시만) / 기준(안 보임) ──
    final HashSet<String> rotCand = new HashSet<>(); // 이번 감시 동안 '주소 바뀐 기준 기기 후보'

    /** 경고에서 빼 주는 것은 '주소가 정확히 같은' 기준뿐. 단 기준에 있던 임의 주소 와이파이(누군가의 핫스팟)는 빼 주지 않는다. */
    boolean exempt(Dev d) {
        if (sessionIgnored.contains(d.key)) return true;          // 이번 시험에서만 무시
        RoomBase sc = rooms.get(ROOM_SCHOOL);
        if (sc != null && sc.keys.contains(d.key)) return true;  // 사람이 확인한 학교 장비
        RoomBase ig = rooms.get(ROOM_IGNORED);
        if (ig != null && ig.keys.contains(d.key)) return true; // 예전 버전에서 '무시'를 누른 것
        if (!baseline.contains(d.key)) return false;
        return !(d.wifi && isRandomMac(d));
    }

    /** '확인 필요' 사유. null 이면 새 신호(경고) */
    String softReason(Dev d, long now) {
        if (d.wifi && d.direct && !d.phoneLike) return "와이파이 다이렉트 · 프린터·TV·화면 공유기일 가능성";
        if (d.wifi && isRandomMac(d) && baseline.contains(d.key)) return "기준 때 있던 핫스팟(개인 기기일 수 있음)";
        String nk = nameKey(d);
        if (nk != null && baseline.contains(nk)) return "이름만 기준과 같음 · 주소는 처음 봄";
        String vk = vendorKey(d);
        if (vk != null && baseline.contains(vk)) return "학교 공유기와 같은 제조사 · 주소는 처음 봄";
        if (rotCand.contains(d.key)) return "주소가 바뀐 기준 기기일 수 있음(추정)";
        if (!d.wifi && d.rank == 0 && !closeAndStaying(d, now)) return d.typeShort + "로 보임 · 시험과 거리 먼 기기일 가능성";
        if (noise(d) && !closeAndStaying(d, now)) return "이름·제조사 없음(TV·비콘·기기 조각 등)";
        return null;
    }

    boolean isAlert(Dev d, long now) { return rawAlert(d, now) && softReason(d, now) == null; }
    boolean isCheck(Dev d, long now) { return rawAlert(d, now) && softReason(d, now) != null; }

    /** 같은 이름(또는 같은 제조사의 이름 없는 기기)은 한 줄로 묶는다 */
    static String groupKey(Dev d) {
        if (d.wifi) return d.key;
        if (d.named) return "N:" + d.name;
        return "M:" + d.manu + ":" + d.typeShort; // 같은 애플이라도 휴대폰과 이어폰은 따로 보인다
    }

    /** 꺼진 기기: 와이파이는 최근 검색에 안 나오면, 블루투스는 8초 동안 소식이 없으면(평소 1초에 여러 번 옴) */
    static boolean isGone(Dev d, long now) {
        if (d.wifi) return d.gone || now - d.lastSeen > GONE_MS;
        return now - d.lastSeen > 8000;
    }

    boolean rawAlert(Dev d, long now) {
        if (mode != MODE_WATCH || exempt(d)) return false;
        if (isGone(d, now)) return false;
        if (d.wifi) return d.ema >= wifiLimit();
        // 블루투스는 3번 이상, 3초 이상 보이고 가까울 때만 (스쳐 가는 신호 거르기)
        return d.hits >= 3 && now - d.firstSeen >= 3_000 && d.ema >= bleLimit();
    }

    /** 신호 세기로 대략 거리(m). 실내에서는 2~3배 오차가 흔하다. */
    static double meters(Dev d) {
        // 1m 에서의 대략 세기. 블루투스가 송신 세기를 알려 주면 그걸로 맞춘다(1m ≈ 송신 세기 - 41)
        double ref = d.wifi ? -40 : (d.txPower >= -30 && d.txPower <= 20 ? d.txPower - 41 : -59);
        double n = d.wifi ? 2.7 : 2.2;     // 실내 감쇠
        double m = Math.pow(10, (ref - d.ema) / (10 * n));
        return Math.max(0.1, Math.min(30, m));
    }

    /** 찾기 화면용 점수: -100dBm=0 ~ -30dBm=100. 숫자가 클수록 가깝다. 거리보다 세밀하게 움직인다. */
    static int score(double ema) { return (int) Math.max(0, Math.min(100, Math.round((ema + 100) * 100 / 70))); }

    /** 짧은 구간(1.5초) 변화: 찾을 때는 빨리 반응해야 한다 */
    static double quickTrend(Dev d) {
        if (d.hist.size() < 2) return 0;
        double[] last = d.hist.get(d.hist.size() - 1);
        for (int i = d.hist.size() - 2; i >= 0; i--) {
            double[] p = d.hist.get(i);
            if (last[0] - p[0] >= 1500) return last[1] - p[1];
        }
        return last[1] - d.hist.get(0)[1];
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
    long lastBatch = 0;
    long lastKick = 0;
    final Runnable tick = new Runnable() {
        @Override public void run() {
            tickN++;
            if (tickN % 15 == 0 && mode == MODE_WATCH) saveLog();
            if (tickN % 60 == 0 && roomsDirty) saveRooms();
            if (surveying && System.currentTimeMillis() >= surveyEnd) finishSurvey();
            long now = System.currentTimeMillis();
            // 안드로이드 9(또는 검색 제한이 켜진 기기)는 2분에 4번만 검색된다.
            // 4초마다 쏘면 앞 16초에 다 쓰고 1분 40초를 기다리게 되니 30초 간격으로 고르게 나눈다.
            // 기준 잡는 1분 동안은 15초 간격(4번)으로 나눠야 제한이 있어도 4번 모두 검색된다
            long gap = (mode == MODE_BASELINE || surveying) ? 6_000 : (Build.VERSION.SDK_INT == 28 || wifiThrottled) ? 30_000 : 4_000;
            if (now - lastKick >= gap) { lastKick = now; kickWifiScan(); }
            // 안드로이드는 30분 넘게 돈 블루투스 검색을 느리게 만든다 → 20분마다 다시 시작
            if (ble != null && now - bleStartedAt > 20 * 60_000L) { stopBle(); startBle(); }
            if (ble == null && hasPerms() && tickN % 5 == 0) startBle();
            // 블루투스 결과가 20초 넘게 끊기면 검색이 멈춘 것으로 보고 다시 시작한다(30초에 한 번까지)
            if (ble != null && lastBleResult > 0 && now - lastBleResult > 20_000 && now - bleStartedAt > 30_000) { stopBle(); startBle(); }
            if (mode == MODE_BASELINE && now >= baselineEnd) finishBaseline();
            if (measuring && (now >= measureEnd || (now - measureStart >= MEASURE_MIN_MS && enoughSamples(now)))) finishMeasure();
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
                .setTitle(patrol ? "기준 잡기 · 복도 순회" : "기준 잡기 · 교실 한 곳")
                .setMessage(patrol
                        ? "학생이 들어오기 전에 해 주세요.\n\n감시할 복도를 처음부터 끝까지 천천히 한 번 걸어 주세요(최소 1분). 걷는 동안 보이는 신호를 모두 '원래 있던 것'으로 저장합니다.\n\n다 걸었으면 '기준 잡기 끝'을 누르세요. 층이 여러 개면 그 층들을 모두 걸은 뒤 누르세요.\n\n한 자리에 서서 잡으면 복도 끝 쪽 학교 공유기가 나중에 새 기기로 뜹니다."
                        : "학생이 들어오기 전, 감시할 그 교실 가운데에서 해 주세요.\n\n1분 동안 이 교실에서 보이는 신호를 '원래 있던 것'으로 저장합니다. 다른 교실로 옮기면 거기서 다시 잡아야 합니다.\n\n감독관 본인 휴대폰은 이때 켜 두면 경고에서 빠집니다.")
                .setPositiveButton("시작", (d, w) -> {
                    baseline.clear(); acked.clear(); devs.clear();
                    mode = MODE_BASELINE;
                    baselineStart = System.currentTimeMillis();
                    // 순회는 사람이 '끝'을 누를 때까지, 교실 한 곳은 1분
                    baselineEnd = patrol ? Long.MAX_VALUE : baselineStart + BASELINE_SECONDS * 1000L;
                    stopBle(); startBle(); kickWifiScan();
                    render(System.currentTimeMillis());
                })
                .setNegativeButton("취소", null).show();
    }

    void finishBaseline() {
        long now = System.currentTimeMillis();
        for (Dev d : devs.values()) if (now - d.lastSeen < GONE_MS) addBase(d);
        saveBaseline();
        mode = MODE_WATCH; rotCand.clear();
        log.clear();
        watchStart = now;
        saveLog();
        buzz(new long[]{0, 120, 80, 120});
    }

    /**
     * 주소와 함께 '이름'도 기준에 넣는다. 블루투스 주소는 15분쯤마다 바뀌지만
     * 이름(예: "Galaxy Buds")은 그대로라, 이름 있는 기기는 주소가 바뀌어도 다시 경고하지 않는다.
     */
    static String nameKey(Dev d) {
        if (d.name == null) return null;
        if (d.wifi) {
            if (d.name.equals("(이름 숨김)")) return null;
            // 임의 주소(휴대폰 핫스팟)는 이름 열쇠를 따로 둔다.
            // 학생이 핫스팟 이름을 학교 와이파이와 똑같이 지어도, 학교 공유기(고유 주소) 이름과는 맞지 않아 경고가 뜬다.
            return (isRandomMac(d) ? "SR:" : "S:") + d.name;
        }
        return d.named ? "N:" + d.name : null;
    }

    /**
     * 학교 공유기는 제조사 고유 주소(앞 3바이트 = 제조사 번호)를 쓰고, 한 대가 2.4/5GHz·여러 이름으로 주소를 여러 개 낸다.
     * 반면 휴대폰 핫스팟은 '임의 주소'(첫 바이트 두 번째 비트가 켜짐)를 쓴다.
     * 그래서 기준에 있던 공유기와 제조사 번호가 같은 고유 주소는 같은 학교 장비로 본다.
     */
    static String vendorKey(Dev d) {
        if (!d.wifi || d.key.length() < 10) return null;
        String mac = d.key.substring(2).toLowerCase(Locale.ROOT);
        try {
            int b0 = Integer.parseInt(mac.substring(0, 2), 16);
            if ((b0 & 0x02) != 0) return null; // 임의 주소(휴대폰 핫스팟 등)는 제외
        } catch (Exception e) { return null; }
        return "O:" + mac.substring(0, 8);
    }

    static boolean isRandomMac(Dev d) {
        try { return (Integer.parseInt(d.key.substring(2, 4), 16) & 0x02) != 0; } catch (Exception e) { return false; }
    }

    void addBase(Dev d) {
        baseline.add(d.key);
        String nk = nameKey(d);
        if (nk != null) baseline.add(nk);
        String vk = vendorKey(d);
        if (vk != null) baseline.add(vk);
    }

    /** 기준 목록의 기기 수(이름 항목은 빼고 센다) */
    int baseCount() { int n = 0; for (String k : baseline) if (k.startsWith("W:") || k.startsWith("B:")) n++; return n; }

    boolean inBase(Dev d) {
        if (baseline.contains(d.key)) return true;
        String nk = nameKey(d);
        if (nk != null && baseline.contains(nk)) return true;
        String vk = vendorKey(d);
        return vk != null && baseline.contains(vk);
    }

    void baseSeen(String key, long now) {
        Long t = baseSeen.get(key);
        if (t == null || now - t > 3600_000L) { baseSeen.put(key, now); roomsDirty = true; }
    }

    void toggleWatch() {
        if (mode == MODE_BASELINE || surveying) return;
        if (mode == MODE_WATCH) {
            mode = MODE_IDLE; stopFlash(); saveLog();
            prefs.edit().putLong("watchStoppedAt", System.currentTimeMillis()).apply();
            return;
        }
        if (baseline.isEmpty()) { toast("먼저 '기준' 탭에서 기준을 재 주세요."); setPage(1); return; }
        if (!hasPerms()) { askPermissions(); return; }
        long now = System.currentTimeMillis();
        long stopped = prefs.getLong("watchStoppedAt", 0);
        // 멈춘 지 30분 안이면 같은 시험으로 보고 그대로 이어 간다(묻지 않음)
        if (!log.isEmpty() && stopped > 0 && now - stopped < 30 * 60_000L) {
            mode = MODE_WATCH; rotCand.clear();
            toast("30분 안에 다시 켜서 지난 감시에 이어서 기록합니다.");
            return;
        }
        String logNote = log.isEmpty() ? "" : "\n\n지난 감시 기록(새 신호 " + log.size() + "건)은 비워집니다. 필요하면 먼저 '기록' 탭에서 저장하세요. 기준은 그대로입니다.";
        new AlertDialog.Builder(this)
                .setTitle("감시 시작")
                .setMessage("학생이 들어오기 전인가요?\n\n· 입실 전: 지금 주변에 보이는 신호(선생님 폰·주변 기기)를 이번 감시에서 빼고 시작합니다.\n· 학생이 이미 있으면 '그냥 시작'을 고르세요(학생 기기까지 빠지면 안 되니까요)." + logNote)
                .setPositiveButton("입실 전 · 주변 정리하고 시작", (d, w) -> beginWatch(true))
                .setNeutralButton("그냥 시작", (d, w) -> beginWatch(false))
                .setNegativeButton("취소", null).show();
    }

    void beginWatch(boolean settle) {
        long now = System.currentTimeMillis();
        log.clear(); rotCand.clear(); sessionIgnored.clear();
        int n = 0;
        if (settle) for (Dev o : devs.values()) if (now - o.lastSeen < 20_000) { sessionIgnored.add(o.key); n++; }
        watchStart = now;
        saveLog();
        mode = MODE_WATCH;
        if (settle) toast("지금 보이는 신호 " + n + "개를 이번 감시에서 뺐습니다.");
        render(now);
    }

    // ───────────────────────── 기록·통계·내보내기 ─────────────────────────

    static class Rec {
        String name, kind; boolean wifi, phone, ignored;
        long first, last, goneAt; double peak = -999; int room = -1;
        int goneKind;        // 1 갑자기 끊김(끈 듯) · 2 약해지다 끊김(멀어진 듯) · 0 불분명
        int flick;           // 끊겼다 다시 잡힌 횟수
        double goneLast;     // 끊기기 직전 세기
        transient Dev cur;
    }
    final java.util.LinkedHashMap<String, Rec> log = new java.util.LinkedHashMap<>();
    long watchStart = 0;
    static final int REQ_SAVE = 9, REQ_IMPORT = 10;

    // ───────────────────────── 다른 태블릿으로 자료 옮기기 ─────────────────────────

    /** 기준(반별)·마지막 본 날·감시 기록·설정을 파일 하나로 */
    String exportJson() throws Exception {
        saveRooms(); saveLog();
        org.json.JSONObject o = new org.json.JSONObject();
        o.put("app", "exam-radar");
        o.put("v", 1);
        o.put("at", System.currentTimeMillis());
        o.put("rooms", new org.json.JSONObject(prefs.getString("rooms", "{}")));
        o.put("baseSeen", new org.json.JSONObject(prefs.getString("baseSeen", "{}")));
        o.put("log", new org.json.JSONArray(prefs.getString("log", "[]")));
        o.put("watchStart", watchStart);
        o.put("places", new org.json.JSONArray(places));
        o.put("labels", new org.json.JSONObject(labels));
        org.json.JSONObject s = new org.json.JSONObject();
        s.put("patrol", patrol); s.put("wide", wide); s.put("sound", sound); s.put("grade", grade); s.put("room", room);
        o.put("settings", s);
        return o.toString();
    }

    void sendToOther() {
        try {
            java.io.File dir = new java.io.File(getCacheDir(), "share");
            dir.mkdirs();
            String name = "시험감독레이더_자료_" + new java.text.SimpleDateFormat("MMdd_HHmm", Locale.KOREA).format(new java.util.Date()) + ".json";
            java.io.File f = new java.io.File(dir, name);
            try (OutputStream o = new java.io.FileOutputStream(f)) { o.write(exportJson().getBytes("UTF-8")); }
            android.net.Uri u = android.net.Uri.parse("content://" + FileShare.AUTH + "/" + android.net.Uri.encode(name));
            Intent i = new Intent(Intent.ACTION_SEND);
            i.setType("application/json");
            i.putExtra(Intent.EXTRA_STREAM, u);
            i.setClipData(android.content.ClipData.newRawUri(name, u));
            i.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
            startActivity(Intent.createChooser(i, "다른 태블릿으로 보내기 (Quick Share 추천)"));
        } catch (Exception e) { toast("자료 파일을 만들지 못했습니다."); }
    }

    void pickImport() {
        Intent i = new Intent(Intent.ACTION_OPEN_DOCUMENT);
        i.addCategory(Intent.CATEGORY_OPENABLE);
        i.setType("*/*");
        try { startActivityForResult(i, REQ_IMPORT); } catch (Exception e) { toast("파일 고르는 화면을 열 수 없습니다."); }
    }

    /** 받은 파일을 열면(또는 고르면) 합칠지 묻는다 */
    void confirmImport(android.net.Uri u) {
        final org.json.JSONObject o;
        try (InputStream in = getContentResolver().openInputStream(u)) {
            java.io.ByteArrayOutputStream bo = new java.io.ByteArrayOutputStream();
            byte[] b = new byte[8192]; int r;
            while ((r = in.read(b)) > 0) bo.write(b, 0, r);
            o = new org.json.JSONObject(bo.toString("UTF-8"));
            if (!"exam-radar".equals(o.optString("app"))) throw new Exception();
        } catch (Exception e) { toast("시험감독 레이더 자료 파일이 아닙니다."); return; }
        int n = o.optJSONObject("rooms") == null ? 0 : o.optJSONObject("rooms").length();
        new AlertDialog.Builder(this)
                .setTitle("자료 가져오기")
                .setMessage("반별 기준 " + n + "곳과 감시 기록, 설정을 가져옵니다.\n\n'합치기'는 이 태블릿에 이미 있는 기준을 지우지 않고 더합니다.")
                .setPositiveButton("합치기", (d, w) -> applyImport(o, false))
                .setNeutralButton("덮어쓰기", (d, w) -> applyImport(o, true))
                .setNegativeButton("취소", null).show();
    }

    void applyImport(org.json.JSONObject o, boolean replace) {
        try {
            if (replace) { rooms.clear(); baseSeen.clear(); log.clear(); }
            org.json.JSONObject rs = o.optJSONObject("rooms");
            if (rs != null) {
                java.util.Iterator<String> it = rs.keys();
                while (it.hasNext()) {
                    String code = it.next();
                    org.json.JSONObject r = rs.getJSONObject(code);
                    int c = Integer.parseInt(code);
                    // 장소(교무실 등)는 순번이 태블릿마다 다를 수 있어 이름으로 다시 맞춘다
                    org.json.JSONArray pl = o.optJSONArray("places");
                    if (c >= 901 && pl != null && c - 901 < pl.length()) c = placeCode(pl.getString(c - 901));
                    RoomBase rb = roomOf(c);
                    rb.t = Math.max(rb.t, r.optLong("t"));
                    org.json.JSONArray ka = r.optJSONArray("k");
                    if (ka != null) for (int i = 0; i < ka.length(); i++) rb.keys.add(ka.getString(i));
                    org.json.JSONObject fp = r.optJSONObject("fp");
                    if (fp != null) { java.util.Iterator<String> fi = fp.keys(); while (fi.hasNext()) { String f = fi.next(); rb.fp.put(f, fp.getDouble(f)); } }
                }
            }
            org.json.JSONObject lbs = o.optJSONObject("labels");
            if (lbs != null) { java.util.Iterator<String> li = lbs.keys(); while (li.hasNext()) { String k = li.next(); labels.put(k, lbs.getString(k)); } saveLabels(); }
            org.json.JSONObject bs = o.optJSONObject("baseSeen");
            if (bs != null) { java.util.Iterator<String> it = bs.keys(); while (it.hasNext()) { String k = it.next(); Long t = baseSeen.get(k); long v = bs.getLong(k); if (t == null || v > t) baseSeen.put(k, v); } }
            org.json.JSONObject s = o.optJSONObject("settings");
            if (s != null) {
                patrol = s.optBoolean("patrol", patrol); wide = s.optBoolean("wide", wide); sound = s.optBoolean("sound", sound);
                grade = s.optInt("grade", grade); room = s.optInt("room", room);
                prefs.edit().putBoolean("patrol", patrol).putBoolean("wide", wide).putBoolean("sound", sound)
                        .putInt("grade", grade).putInt("room", room).apply();
            }
            if (replace || log.isEmpty()) {
                prefs.edit().putString("log", o.optJSONArray("log") == null ? "[]" : o.optJSONArray("log").toString())
                        .putLong("watchStart", o.optLong("watchStart", 0)).apply();
                log.clear();
                loadLog();
            }
            rebuildBaseline();
            saveRooms();
            prefs.edit().putBoolean("askedImport", true).apply();
            toast("가져왔습니다. 기준 " + baseCount() + "개, " + rooms.size() + "곳.");
            setPage(1);
        } catch (Exception e) { toast("가져오지 못했습니다."); }
    }

    /** 받은 파일을 눌러 이 앱으로 열었을 때 */
    void handleIncoming(Intent it) {
        if (it == null) return;
        android.net.Uri u = null;
        if (Intent.ACTION_VIEW.equals(it.getAction())) u = it.getData();
        else if (Intent.ACTION_SEND.equals(it.getAction())) {
            @SuppressWarnings("deprecation") android.net.Uri s = it.getParcelableExtra(Intent.EXTRA_STREAM);
            u = s;
        }
        if (u != null) confirmImport(u);
    }

    @Override
    protected void onNewIntent(Intent it) {
        super.onNewIntent(it);
        handleIncoming(it);
    }

    void logAlert(Dev d, long now) {
        String k = groupKey(d);
        Rec r = log.get(k);
        if (r == null) { r = new Rec(); r.first = now; log.put(k, r); }
        r.name = d.name; r.kind = d.kind; r.wifi = d.wifi; r.phone = d.phoneLike;
        r.last = Math.max(r.last, d.lastSeen);
        if (d.ema > r.peak) r.peak = d.ema;
        int br = bestRoom(d);
        if (br > 0) r.room = br;
    }

    void saveLog() {
        try {
            org.json.JSONArray a = new org.json.JSONArray();
            for (Map.Entry<String, Rec> en : log.entrySet()) {
                Rec r = en.getValue();
                org.json.JSONObject o = new org.json.JSONObject();
                o.put("g", en.getKey());
                o.put("n", r.name); o.put("k", r.kind); o.put("w", r.wifi); o.put("p", r.phone); o.put("i", r.ignored);
                o.put("f", r.first); o.put("l", r.last); o.put("pk", r.peak); o.put("r", r.room); o.put("x", r.goneAt);
                o.put("gk", r.goneKind); o.put("fl", r.flick); o.put("gl", r.goneLast);
                a.put(o);
            }
            prefs.edit().putString("log", a.toString()).putLong("watchStart", watchStart).apply();
        } catch (Exception ignored) { }
    }

    void loadLog() {
        try {
            watchStart = prefs.getLong("watchStart", 0);
            org.json.JSONArray a = new org.json.JSONArray(prefs.getString("log", "[]"));
            for (int i = 0; i < a.length(); i++) {
                org.json.JSONObject o = a.getJSONObject(i);
                Rec r = new Rec();
                r.name = o.optString("n"); r.kind = o.optString("k"); r.wifi = o.optBoolean("w"); r.phone = o.optBoolean("p");
                r.ignored = o.optBoolean("i"); r.first = o.optLong("f"); r.last = o.optLong("l");
                r.peak = o.optDouble("pk", -999); r.room = o.optInt("r", -1); r.goneAt = o.optLong("x", 0);
                r.goneKind = o.optInt("gk", 0); r.flick = o.optInt("fl", 0); r.goneLast = o.optDouble("gl", -999);
                // 0.15 이전 기록의 반 정보는 실제로 재지 않고 '선택된 반'이 들어간 것이라 한 번 비운다
                if (!prefs.getBoolean("logRoomFix1", false)) r.room = -1;
                log.put(o.optString("g", (r.wifi ? "W" : "B") + i), r);
            }
            if (!prefs.getBoolean("logRoomFix1", false)) { prefs.edit().putBoolean("logRoomFix1", true).apply(); saveLog(); }
        } catch (Exception ignored) { }
    }

    static String hm(long t) { return new java.text.SimpleDateFormat("HH:mm", Locale.KOREA).format(new java.util.Date(t)); }

    String summary() {
        if (watchStart == 0) return "아직 감시 기록이 없습니다. 기준을 잡고 감시를 시작하면 기록됩니다.";
        long end = System.currentTimeMillis();
        int w = 0, b = 0, ph = 0, ig = 0;
        Map<Integer, Integer> byRoom = new java.util.TreeMap<>();
        for (Rec r : log.values()) {
            if (r.wifi) w++; else b++;
            if (r.phone) ph++;
            if (r.ignored) ig++;
            if (r.room > 0) byRoom.merge(r.room, 1, Integer::sum);
        }
        StringBuilder s = new StringBuilder();
        s.append("감시 시간: ").append(hm(watchStart)).append(" ~ ").append(hm(end))
                .append(" (").append((end - watchStart) / 60000).append("분)\n");
        s.append("기준 기기: ").append(baseCount()).append("개\n\n");
        s.append("새로 나타난 기기: ").append(log.size()).append("개\n");
        s.append(" · 와이파이 핫스팟 ").append(w).append("개\n");
        s.append(" · 블루투스 ").append(b).append("개\n");
        s.append(" · 휴대폰 의심 ").append(ph).append("개\n");
        s.append(" · '무시' 처리 ").append(ig).append("개\n");
        if (!byRoom.isEmpty()) {
            s.append("\n교실별(순회):\n");
            for (Map.Entry<Integer, Integer> e : byRoom.entrySet())
                s.append(" · ").append(roomName(e.getKey())).append(" 앞 ").append(e.getValue()).append("개\n");
        }
        if (!log.isEmpty()) {
            s.append("\n처음 본 순서:\n");
            int n = 0;
            for (Rec r : log.values()) {
                if (++n > 15) { s.append(" … 나머지는 파일에 있습니다\n"); break; }
                s.append(" ").append(hm(r.first)).append(" ").append(r.wifi ? "[와이파이] " : "[블루투스] ").append(r.name)
                        .append(r.room > 0 ? " · " + roomName(r.room) + " 앞" : "").append(r.ignored ? " (무시)" : "").append("\n");
            }
        }
        return s.toString();
    }

    String csv() {
        StringBuilder s = new StringBuilder("﻿");
        s.append("번호,처음 본 시각,마지막 본 시각,머문 시간(분),종류,이름,설명,가장 센 세기(dBm),가장 셌던 곳,처리,끊긴 시각,끊긴 모양\n");
        int n = 0;
        java.text.SimpleDateFormat f = new java.text.SimpleDateFormat("yyyy-MM-dd HH:mm:ss", Locale.KOREA);
        for (Rec r : log.values()) {
            s.append(++n).append(',').append(f.format(new java.util.Date(r.first))).append(',')
                    .append(f.format(new java.util.Date(r.last))).append(',')
                    .append(Math.max(0, (r.last - r.first) / 60000)).append(',')
                    .append(r.wifi ? "와이파이" : "블루투스").append(',')
                    .append(q(r.name)).append(',').append(q(r.kind)).append(',')
                    .append(Math.round(r.peak)).append(',')
                    .append(r.room > 0 ? roomName(r.room) + " 앞" : "").append(',')
                    .append(r.ignored ? "무시" : "").append(',')
                    .append(r.goneAt > 0 ? f.format(new java.util.Date(r.goneAt)) : "").append(',')
                    .append(r.goneAt > 0 || r.flick > 0 ? q(goneWord(r)) : "").append('\n');
        }
        return s.toString();
    }

    static String q(String v) { return "\"" + (v == null ? "" : v.replace("\"", "\"\"")) + "\""; }

    void showStats() {
        saveLog();
        new AlertDialog.Builder(this)
                .setTitle("감시 기록")
                .setMessage(summary())
                .setPositiveButton("파일로 저장", (d, w) -> {
                    Intent i = new Intent(Intent.ACTION_CREATE_DOCUMENT);
                    i.addCategory(Intent.CATEGORY_OPENABLE);
                    i.setType("text/csv");
                    i.putExtra(Intent.EXTRA_TITLE, "시험감독기록_" + new java.text.SimpleDateFormat("yyyyMMdd_HHmm", Locale.KOREA).format(new java.util.Date()) + ".csv");
                    try { startActivityForResult(i, REQ_SAVE); } catch (Exception e) { toast("파일 저장 화면을 열 수 없습니다."); }
                })
                .setNeutralButton("공유", (d, w) -> {
                    Intent i = new Intent(Intent.ACTION_SEND);
                    i.setType("text/plain");
                    i.putExtra(Intent.EXTRA_SUBJECT, "시험감독 레이더 기록");
                    i.putExtra(Intent.EXTRA_TEXT, summary());
                    startActivity(Intent.createChooser(i, "기록 보내기"));
                })
                .setNegativeButton("닫기", null).show();
    }

    @Override
    protected void onActivityResult(int code, int res, Intent data) {
        super.onActivityResult(code, res, data);
        if (code == REQ_IMPORT && res == RESULT_OK && data != null && data.getData() != null) { confirmImport(data.getData()); return; }
        if (code != REQ_SAVE || res != RESULT_OK || data == null || data.getData() == null) return;
        try (OutputStream o = getContentResolver().openOutputStream(data.getData())) {
            o.write(csv().getBytes("UTF-8"));
            toast("저장했습니다. 엑셀에서 바로 열립니다.");
        } catch (Exception e) { toast("저장하지 못했습니다."); }
    }

    void togglePatrol() {
        patrol = !patrol;
        prefs.edit().putBoolean("patrol", patrol).apply();
        new AlertDialog.Builder(this)
                .setTitle(patrol ? "복도 순회 모드" : "교실 고정 모드")
                .setMessage(patrol
                        ? "학생 입실 전에 복도를 끝까지 한 번 걸으며 기준을 다시 잡아 주세요.\n\n시험 중에는 반 앞마다 멈춰 '여기서 재기'를 누르세요. 새 기기마다 가장 센 반을 알려 드립니다."
                        : "교실 가운데에 태블릿을 두는 방식입니다. 그 교실에서 기준을 다시 잡아 주세요.")
                .setPositiveButton("기준 탭으로", (d, w) -> setPage(1))
                .setNegativeButton("나중에", null).show();
    }

    void setRoom(int r) {
        room = Math.max(1, Math.min(grade == 9 ? Math.max(1, places.size()) : 30, r));
        prefs.edit().putInt("room", room).apply();
        render(System.currentTimeMillis());
    }

    void ignore(Dev d) {
        // 한 줄로 묶인 기기(예: "삼성 기기 ×3")는 묶음 전체를 무시한다
        String g = groupKey(d);
        RoomBase igRoom = roomOf(ROOM_IGNORED);
        igRoom.t = System.currentTimeMillis();
        for (Dev o : devs.values()) if (groupKey(o).equals(g)) { addBase(o); igRoom.keys.add(o.key); }
        igRoom.keys.add(d.key);
        addBase(d);
        saveBaseline();
        Rec r = log.get(groupKey(d));
        if (r != null) { r.ignored = true; saveLog(); }
        if (d.key.equals(findKey)) closeFinder();
    }

    void ackAll() {
        long now = System.currentTimeMillis();
        for (Dev d : devs.values()) if (isAlert(d, now)) acked.add(d.key);
        stopFlash();
    }

    /** 무시 등으로 기준에 새로 들어간 열쇠는 '직접 무시한 기기' 칸에 넣어 저장한다 */
    void saveBaseline() {
        HashSet<String> known = new HashSet<>();
        for (RoomBase rb : rooms.values()) known.addAll(rb.keys);
        long now = System.currentTimeMillis();
        for (String k : baseline) if (!known.contains(k)) { RoomBase ig = roomOf(ROOM_IGNORED); ig.keys.add(k); ig.t = now; baseSeen.put(k, now); }
        saveRooms();
    }

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
        if (page == 1) renderBasePane(now);
        if (page == 2) logText.setText(summary());
        List<Dev> alerts = new ArrayList<>();
        int baseSeen = 0;
        for (Dev d : devs.values()) {
            // 기록에 오른 새 신호가 끊기면 '사라진 신호'로 남긴다
            if (mode == MODE_WATCH) {
                Rec rr = log.get(groupKey(d));
                if (rr != null) {
                    if (isGone(d, now)) {
                        if (rr.goneAt == 0) { rr.goneAt = d.lastSeen; rr.goneKind = goneKind(d); rr.goneLast = d.ema; }
                    } else if (rr.goneAt > 0) { rr.goneAt = 0; rr.flick++; } // 끊겼다가 다시 잡힘
                    rr.cur = d;
                }
            }
            if (isAlert(d, now)) { alerts.add(d); logAlert(d, now); }
            else if (baseline.contains(d.key) && now - d.lastSeen < GONE_MS) {
                baseSeen++;
                // 기준 기기가 다시 보이면 '마지막으로 본 날'을 갱신해 30일 정리에서 살린다
                if (now - d.lastSeen < 5000) { baseSeen(d.key, now); }
            }
        }
        // '가까이 오래 있음' → 휴대폰 → 이어폰·시계 → 나머지 순, 같은 칸 안에서는 센 순
        alerts.sort((a, b) -> priorityCmp(a, b, now));
        Map<String, Integer> gCount = new HashMap<>();
        List<Dev> shown = new ArrayList<>();
        for (Dev d : alerts) {
            String g = groupKey(d);
            Integer c = gCount.get(g);
            if (c == null) { gCount.put(g, 1); shown.add(d); } else gCount.put(g, c + 1);
        }
        int unacked = 0;
        for (Dev d : alerts) if (!acked.contains(d.key)) unacked++;

        // 검색 상태: 블루투스는 평소 1초에도 여러 번 들어온다. 와이파이는 제한 때문에 느리다.
        BluetoothAdapter adH = btAdapter();
        boolean btOn = adH != null && adH.isEnabled();
        boolean bleOk = btOn && bleFail == 0 && now - lastBleResult < 20_000;
        long wifiLimitMs = (Build.VERSION.SDK_INT == 28 || wifiThrottled) ? 90_000 : 30_000;
        boolean wifiOk = wifi != null && wifi.isWifiEnabled() && lastWifiFresh > 0 && now - lastWifiFresh < wifiLimitMs;
        healthy = bleOk && wifiOk;

        if (mode == MODE_BASELINE) {
            if (patrol) {
                long el = (now - baselineStart) / 1000;
                status.setText("기준 잡는 중 · " + el + "초째");
                statusSub.setText("복도를 끝까지 천천히 걸어 주세요. 지금까지 신호 " + baseCount() + "개를 기억했습니다."
                        + (el < 60 ? " (최소 1분)" : " 다 걸었으면 '기준 잡기 끝'을 누르세요."));
            } else {
                long left = Math.max(0, (baselineEnd - now + 999) / 1000);
                status.setText("기준 잡는 중 · " + left + "초");
                statusSub.setText("이 교실에서 보이는 신호 " + baseCount() + "개를 기억하고 있습니다. 태블릿을 교실 가운데쯤 두세요.");
            }
            status.setTextColor(LIME);
            stopFlash();
        } else if (mode == MODE_WATCH) {
            if (alerts.isEmpty()) {
                // 아무것도 안 잡힌 것과 '기기가 없다'는 다르다: 검색이 멈췄으면 그렇다고 말한다
                status.setText(healthy ? "새 신호 없음" : "검색 멈춤 · 확인 필요");
                status.setTextColor(healthy ? LIME : ORANGE);
                statusSub.setText(healthy ? "지금은 새 신호가 안 보입니다. 전원이나 무선을 끈 기기는 잡히지 않습니다."
                        : "검색이 제대로 안 되고 있어 '새 신호 없음'을 믿으면 안 됩니다. 아래 상태줄을 보세요.");
                stopFlash();
            } else {
                int phones = 0;
                for (Dev d : shown) if (d.phoneLike) phones++;
                status.setText("새 신호 " + shown.size() + "개" + (phones > 0 ? " · 휴대폰 의심 " + phones : ""));
                status.setTextColor(WHITE);
                statusSub.setText(patrol ? "반 앞에 멈춰 '여기서 재기'를 누르면, 잰 곳 중 어디가 강한지 알려 드립니다."
                        : unacked > 0 ? "목록에서 눌러 거리를 보며 찾으세요." : "확인함. 새로 생기면 다시 깜빡입니다.");
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
        btnClearAll.setVisibility(mode == MODE_WATCH && shown.size() >= 2 ? View.VISIBLE : View.GONE);
        if (mode == MODE_BASELINE && patrol) {
            boolean ok = now - baselineStart >= 60_000;
            btnBase.setText(ok ? "기준 잡기 끝" : "걷는 중… (1분 뒤 끝내기)");
            btnBase.setEnabled(ok);
            styleBtn(btnBase, ok ? ORANGE : NAVY2, WHITE);
        } else {
            btnBase.setText(patrol ? "① 기준 잡기 (복도 한 바퀴)" : "① 기준 잡기 (이 교실, 1분)");
            btnBase.setEnabled(mode != MODE_BASELINE);
            styleBtn(btnBase, NAVY2, WHITE);
        }
        btnWatch.setText(mode == MODE_WATCH ? "감시 멈춤" : "감시 시작");
        styleBtn(btnWatch, mode == MODE_WATCH ? NAVY2 : LIME, mode == MODE_WATCH ? WHITE : NAVY);
        btnRange.setText(wide ? "경고 범위: 넓게" : "경고 범위: 교실 안");
        btnSound.setText(sound ? "소리: 켬" : "소리: 끔 (진동만)");
        btnPatrol.setText(patrol ? "복도 순회 모드 켜짐 (누르면 교실 고정으로)" : "복도 순회 모드로 바꾸기");
        styleBtn(btnPatrol, patrol ? SKY : NAVY2, patrol ? NAVY : WHITE);
        btnRange.setVisibility(patrol ? View.GONE : View.VISIBLE);
        patrolBox.setVisibility(patrol ? View.VISIBLE : View.GONE);
        if (patrol) {
            roomText.setText(roomName(loc()) + (grade == 9 ? "" : " 앞"));
            styleGradeButtons(gradeBtns);
            if (measuring) {
                btnMeasure.setText("재는 중 " + (now - measureStart) / 1000 + "초 · 가만히 서 계세요 (5~15초)");
                btnMeasure.setEnabled(false);
                styleBtn(btnMeasure, LIME, NAVY);
            } else {
                btnMeasure.setText("여기서 재기");
                btnMeasure.setEnabled(true);
                styleBtn(btnMeasure, SKY, NAVY);
            }
            Dev top = alerts.isEmpty() ? null : alerts.get(0);
            if (top == null) { topText.setText("새 신호 없음"); topText.setTextColor(LIME); }
            else {
                topText.setText("가장 센 새 신호: " + top.name + "\n" + distText(meters(top)) + " · " + trendWord(top)
                        + " · " + Math.round(top.ema) + " dBm");
                topText.setTextColor(meters(top) < 4 ? ORANGE : WHITE);
            }
        }

        baseInfo.setText("● 블루투스 " + (bleOk ? "정상" : !btOn ? "꺼짐" : "멈춤")
                + "   ● 와이파이 " + (wifiOk ? "정상 (" + (now - lastWifiFresh) / 1000 + "초 전 갱신)" : wifi != null && !wifi.isWifiEnabled() ? "꺼짐" : "늦음")
                + "   · 기준 " + baseCount() + "개");
        baseInfo.setTextColor(healthy ? LIME : ORANGE);
        String t = "";
        if (Build.VERSION.SDK_INT >= 29 && wifiThrottled && now - lastWifiScanOk > 40_000)
            t = "와이파이 검색이 느립니다. 설정 → 개발자 옵션 → 'Wi-Fi 스캔 쓰로틀링'(Wi-Fi 검색 제한)을 끄면 4초마다 갱신됩니다.";
        else if (lastWifiResult > 0 && now - lastWifiResult > 60_000)
            t = "1분 넘게 와이파이 결과가 없습니다. 와이파이 스위치와 위치가 켜져 있는지 보세요.";
        tip.setText(t);
        tip.setVisibility(t.isEmpty() ? View.GONE : View.VISIBLE);

        list.removeAllViews();
        List<Dev> wl = new ArrayList<>(), bl = new ArrayList<>();
        for (Dev d : shown) (d.wifi ? wl : bl).add(d);
        tabWifi.setText("와이파이 " + wl.size());
        tabBle.setText("블루투스 " + bl.size());
        styleBtn(tabWifi, tab == 1 ? SKY : NAVY2, tab == 1 ? NAVY : SKY);
        styleBtn(tabBle, tab == 2 ? LIME : NAVY2, tab == 2 ? NAVY : LIME);
        styleBtn(tabAll, tab == 0 ? WHITE : NAVY2, tab == 0 ? NAVY : WHITE);
        if (tab != 2) {
            if (tab == 0) list.addView(section("와이파이 핫스팟 " + wl.size() + "개", SKY));
            if (wl.isEmpty()) list.addView(label("  새 와이파이 없음", 14, DIM, false));
            for (Dev d : wl) list.addView(row(d, now, gCount.get(groupKey(d))));
        }
        if (tab != 1) {
            if (tab == 0) list.addView(section("블루투스 " + bl.size() + "개", LIME));
            if (bl.isEmpty()) list.addView(label("  새 블루투스 없음", 14, DIM, false));
            for (Dev d : bl) list.addView(row(d, now, gCount.get(groupKey(d))));
        }
        // ── 확인 필요(경고 아님): 접어 두고 한 줄로 ──
        List<Dev> checks = new ArrayList<>();
        for (Dev d : devs.values()) if (isCheck(d, now) && (tab == 0 || (tab == 1) == d.wifi)) checks.add(d);
        checks.sort((a, b) -> priorityCmp(a, b, now));
        if (!checks.isEmpty()) {
            int stars = 0;
            for (Dev d : checks) if (nearStay(d, now)) stars++;
            list.addView(foldHeader("확인 필요 " + checks.size() + "개" + (stars > 0 ? " · ★ 먼저 볼 것 " + stars + "개" : ""),
                    "경고 아님 · ★ 붙은 것(가까이 오래 있음)만 먼저 보세요", showUnknown, SKY,
                    v -> { showUnknown = !showUnknown; render(System.currentTimeMillis()); }));
            // 접혀 있어도 ★ 붙은 것은 보여 준다(놓치지 않게)
            for (Dev d : checks) if (showUnknown || nearStay(d, now)) list.addView(row(d, now, 1));
        }
        // ── 사라진 신호: 감시 중 잡혔다가 끊긴 새 신호(감독관이 다가가자 끈 경우 등) ──
        List<Rec> goneList = new ArrayList<>();
        for (Rec r : log.values()) if (r.goneAt > 0 && now - r.goneAt < 30 * 60_000L && !r.ignored) goneList.add(r);
        goneList.sort((a, b) -> Long.compare(b.goneAt, a.goneAt));
        if (!goneList.isEmpty()) {
            int off = 0;
            for (Rec r : goneList) if (r.goneKind == 1 && r.flick < 2) off++;
            list.addView(foldHeader("사라진 신호 " + goneList.size() + "개" + (off > 0 ? " · 끈 것 같음 " + off + "개" : ""),
                    "잡혔다가 끊김 · 끊긴 모양으로 '끈 듯/멀어진 듯'을 구분", showGone || off > 0, ORANGE,
                    v -> { showGone = !showGone; render(System.currentTimeMillis()); }));
            // '끈 것 같음'이 있으면 펼쳐서 보여 준다(놓치지 않게)
            if (showGone || off > 0) for (Rec r : goneList) {
                if (!showGone && !(r.goneKind == 1 && r.flick < 2)) continue;
                boolean likelyOff = r.goneKind == 1 && r.flick < 2;
                LinearLayout g = new LinearLayout(this);
                g.setOrientation(LinearLayout.VERTICAL);
                g.setPadding(dp(14), dp(10), dp(14), dp(10));
                g.setBackground(round(NAVY2, 12, likelyOff ? ORANGE : NAVY2));
                g.addView(label(goneWord(r), 15, likelyOff ? ORANGE : SKY, true));
                g.addView(label((r.wifi ? "와이파이 · " : "블루투스 · ") + r.name, 16, WHITE, true));
                g.addView(label(hm(r.goneAt) + "에 끊김" + (r.room > 0 ? " · " + roomName(r.room) + " 쪽에서 강했음" : "")
                        + (likelyOff ? "\n그 근처에 다시 가서 안 잡히면 끈 것, 다시 잡히면 거리 때문입니다." : ""), 13, DIM, false));
                LinearLayout.LayoutParams glp2 = new LinearLayout.LayoutParams(-1, -2);
                glp2.bottomMargin = dp(8);
                list.addView(g, glp2);
            }
        }
    }

    boolean showGone = false;

    /** 먼저 볼 것: 약 3m 안(블루투스 -65·와이파이 -60dBm 이상)으로 1분 넘게 계속 잡힘 */
    static boolean nearStay(Dev d, long now) {
        return d.ema >= (d.wifi ? -60 : -65) && now - d.firstSeen >= 60_000 && now - d.lastSeen < 10_000;
    }

    static int prio(Dev d, long now) {
        int r = d.wifi ? (d.phoneLike ? 3 : 2) : d.rank;
        return (nearStay(d, now) ? 10 : 0) + r;
    }

    static int priorityCmp(Dev a, Dev b, long now) {
        int pa = prio(a, now), pb = prio(b, now);
        return pa != pb ? Integer.compare(pb, pa) : Double.compare(b.ema, a.ema);
    }

    /**
     * 끊긴 모양 판정.
     * 끈 듯: 끊기기 직전 세기가 잡히는 한계보다 12dB 넘게 강했고, 그 전 6초 동안 크게 약해지지 않았다.
     * 멀어진 듯: 직전 세기가 한계 근처(8dB 안)이거나, 6초 동안 6dB 넘게 약해지고 있었다.
     */
    int goneKind(Dev d) {
        // 순회 중 마지막으로 잡힌 뒤 감독관이 다른 반으로 옮겼으면 거리 때문일 가능성이 크다
        if (patrol && d.lastLoc != 0 && d.lastLoc != loc()) return 3;
        double lim = d.wifi ? wifiLimit() : bleLimit();
        double last = d.ema;
        double prevMax = last;
        for (double[] p : d.hist) if (d.lastSeen - p[0] <= 6000) prevMax = Math.max(prevMax, p[1]);
        double drop = prevMax - last;
        if (last - lim < 8 || drop > 6) return 2;
        // 와이파이는 갱신이 느려 걷기만 해도 '갑자기'처럼 보일 수 있어 더 엄격하게(한계보다 18dB 이상 강했을 때만)
        if (last - lim >= (d.wifi ? 18 : 12) && drop <= 4) return 1;
        return 0;
    }

    static String goneWord(Rec r) {
        if (r.flick >= 2) return "끊겼다 잡혔다 " + r.flick + "번 · 경계 거리(멀어서 그럴 가능성)";
        if (r.goneKind == 1) return "세게 잡히다 갑자기 끊김 · 끈 것 같음";
        if (r.goneKind == 2) return "약해지다 끊김 · 멀어진 것 같음";
        if (r.goneKind == 3) return "감독관이 다른 반으로 옮긴 뒤 끊김 · 거리 때문일 가능성";
        return "끊김 · 이유 불분명";
    }

    /** 접는 칸 머리: "확인 필요 3개  ▼ 펼치기" + 한 줄 설명 */
    View foldHeader(String title, String sub, boolean open, int color, View.OnClickListener click) {
        LinearLayout h = new LinearLayout(this);
        h.setOrientation(LinearLayout.VERTICAL);
        h.setPadding(dp(14), dp(12), dp(14), dp(12));
        h.setBackground(round(NAVY, 12, color));
        h.addView(label(title + (open ? "   ▲ 접기" : "   ▼ 펼치기"), 16, color, true));
        h.addView(label(sub, 13, DIM, false));
        h.setOnClickListener(click);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(-1, -2);
        lp.topMargin = dp(14);
        lp.bottomMargin = dp(8);
        h.setLayoutParams(lp);
        return h;
    }

    View section(String title, int color) {
        TextView t = label(title, 16, color, true);
        t.setPadding(dp(4), dp(14), 0, dp(8));
        return t;
    }

    View row(Dev d, long now, int count) {
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

        // 한눈에 보이게: 블루투스는 종류(휴대폰·이어폰 등)를 이름표로
        String bt = d.wifi ? (d.phoneLike ? "핫스팟" : "와이파이") : (d.typeShort != null ? d.typeShort : "블루투스");
        TextView badge = label(bt, 13, NAVY, true);
        badge.setBackground(round(d.phoneLike ? ORANGE : d.wifi ? SKY : LIME, 8));
        badge.setPadding(dp(8), dp(4), dp(8), dp(4));
        r.addView(badge);

        LinearLayout mid = new LinearLayout(this);
        mid.setOrientation(LinearLayout.VERTICAL);
        mid.setPadding(dp(12), 0, dp(8), 0);
        mid.addView(label(d.name + (count > 1 ? "  ×" + count : ""), 18, WHITE, true));
        long ago = (now - d.lastSeen) / 1000;
        mid.addView(label(d.kind + (ago > 5 ? " · " + ago + "초 전" : ""), 13, DIM, false));
        if (nearStay(d, now)) mid.addView(label("★ 가까이 오래 있음 · 먼저 확인", 15, ORANGE, true));
        String why = softReason(d, now);
        if (why != null) mid.addView(label(why, 14, SKY, false));
        String vd = patrol ? verdict(d) : null;
        if (vd != null) mid.addView(label(vd, 15, vd.contains("뚜렷") ? ORANGE : SKY, true));
        r.addView(mid, new LinearLayout.LayoutParams(0, -2, 1));

        double m = meters(d);
        LinearLayout right = new LinearLayout(this);
        right.setOrientation(LinearLayout.VERTICAL);
        right.setGravity(Gravity.END);
        right.addView(label(distText(m), 20, m < 3 ? ORANGE : WHITE, true));
        right.addView(label(zone(m), 12, DIM, false));
        r.addView(right);

        Button ig = btn("처리", NAVY, SKY);
        ig.setTextSize(14);
        LinearLayout.LayoutParams ilp = new LinearLayout.LayoutParams(-2, dp(44));
        ilp.leftMargin = dp(10);
        ig.setLayoutParams(ilp);
        ig.setOnClickListener(v -> handle(d));
        r.addView(ig);

        r.setOnClickListener(v -> openFinder(d.key));
        return r;
    }

    // ───────────────────────── 가까이 찾기: 최고점 기억·방향 찾기 ─────────────────────────

    double findPeak = -999;
    android.hardware.SensorManager sm;
    float azimuth = 0;
    boolean dirScanning = false;
    long dirEnd = 0;
    final double[] binSum = new double[12];
    final int[] binN = new int[12];
    double bestAz = Double.NaN;
    TextView arrow, dirText;
    Button btnDir;
    final float[] rotM = new float[9], ori = new float[3];

    final android.hardware.SensorEventListener rotL = new android.hardware.SensorEventListener() {
        @Override public void onSensorChanged(android.hardware.SensorEvent e) {
            android.hardware.SensorManager.getRotationMatrixFromVector(rotM, e.values);
            android.hardware.SensorManager.getOrientation(rotM, ori);
            azimuth = (float) ((Math.toDegrees(ori[0]) + 360) % 360);
            if (arrow != null && !Double.isNaN(bestAz)) arrow.setRotation((float) (bestAz - azimuth));
        }
        @Override public void onAccuracyChanged(android.hardware.Sensor s, int a) { }
    };

    android.hardware.Sensor rotSensor() {
        if (sm == null) sm = (android.hardware.SensorManager) getSystemService(Context.SENSOR_SERVICE);
        return sm == null ? null : sm.getDefaultSensor(android.hardware.Sensor.TYPE_ROTATION_VECTOR);
    }

    /** 몸이 등 뒤 신호를 가리는 것을 이용: 태블릿을 가슴 앞에 들고 한 바퀴 돌면 가장 센 쪽이 기기 방향 */
    double prevAz = Double.NaN; // 첫 바퀴 결과(두 번 같아야 화살표를 보여 준다)

    void startDir() {
        if (rotSensor() == null) { toast("이 기기는 나침반 센서가 없어 방향 찾기를 할 수 없습니다."); return; }
        Dev fd = devs.get(findKey);
        if (fd != null && fd.wifi) { toast("와이파이는 갱신이 느려 방향 찾기가 안 됩니다. 신호 점수가 커지는 쪽으로 걸어 보세요."); return; }
        java.util.Arrays.fill(binSum, 0); java.util.Arrays.fill(binN, 0);
        bestAz = Double.NaN;
        arrow.setVisibility(View.INVISIBLE);
        dirScanning = true;
        dirEnd = System.currentTimeMillis() + 12_000;
        kickWifiScan();
    }

    void finishDir() {
        dirScanning = false;
        int covered = 0, best = -1; double bv = -999;
        for (int i = 0; i < 12; i++) {
            if (binN[i] == 0) continue;
            covered++;
            double avg = binSum[i] / binN[i];
            if (avg > bv) { bv = avg; best = i; }
        }
        if (covered < 6 || best < 0) {
            dirText.setText("한 바퀴를 다 돌지 못했거나 신호가 적었습니다. 더 천천히 다시 돌아 주세요.");
            return;
        }
        double az = best * 30 + 15;
        // 반사·몸 가림 때문에 한 번 결과는 틀릴 수 있다: 두 바퀴가 60도 안으로 같을 때만 화살표
        if (Double.isNaN(prevAz)) {
            prevAz = az;
            dirText.setText("한 바퀴 끝. 같은 자리에서 한 번 더 돌아 주세요. 두 번 결과가 같아야 화살표를 보여 드립니다.");
            buzz(new long[]{0, 60});
            return;
        }
        double diff = Math.abs(((az - prevAz) % 360 + 540) % 360 - 180);
        if (diff > 60) {
            prevAz = az;
            dirText.setText("두 번의 방향이 다릅니다(반사가 심한 곳). 몇 걸음 옮겨 다시 돌거나, 신호 점수가 커지는 쪽으로 걸어 보세요.");
            return;
        }
        bestAz = Math.toDegrees(Math.atan2(Math.sin(Math.toRadians(az)) + Math.sin(Math.toRadians(prevAz)),
                Math.cos(Math.toRadians(az)) + Math.cos(Math.toRadians(prevAz))));
        prevAz = Double.NaN;
        arrow.setVisibility(View.VISIBLE);
        dirText.setText("두 번 같은 방향이 나왔습니다. 화살표 쪽으로 몇 걸음 간 뒤 다시 확인하세요(대략적인 방향입니다).");
        buzz(new long[]{0, 80, 60, 80});
    }

    /** 가까이에서는 0.5m 단위로 보여 준다(그래도 신호 세기 기반이라 대략값) */
    static String fineDist(double m) {
        if (m < 0.2) return "약 20cm 안";
        if (m < 1) return "약 " + Math.round(m * 10) * 10 + "cm";
        if (m < 3) return String.format(Locale.KOREA, "약 %.1fm", Math.round(m * 2) / 2.0);
        return distText(m);
    }

    String peakLine(Dev d) {
        double cur = Double.isNaN(findShow) ? d.ema : findShow;
        if (cur > findPeak) findPeak = cur;
        double gap = findPeak - cur;
        return "최고 " + Math.round(findPeak) + " · 지금 " + Math.round(cur) + " dBm  →  "
                + (gap <= 2 ? "가장 가까웠던 지점 근처입니다" : "최고점보다 " + Math.round(gap) + "dB 약함, 왔던 쪽으로 돌아가 보세요") + "\n";
    }

    final Runnable fastFind = new Runnable() {
        @Override public void run() {
            if (findKey == null) return;
            long now = System.currentTimeMillis();
            if (dirScanning && now >= dirEnd) finishDir();
            renderFinder(now);
            h.postDelayed(this, 300);
        }
    };

    void openFinder(String key) {
        findKey = key;
        findPeak = -999;
        findShow = Double.NaN;
        findTrack.clear();
        bestAz = Double.NaN;
        prevAz = Double.NaN;
        dirScanning = false;
        if (arrow != null) arrow.setVisibility(View.INVISIBLE);
        if (dirText != null) dirText.setText("의심 기기 쪽을 모르면: 태블릿을 화면이 위를 보게 가슴 앞에 평평하게 들고 '방향 찾기'를 누른 뒤 제자리에서 천천히 한 바퀴 도세요.");
        android.hardware.Sensor rs = rotSensor();
        if (rs != null) sm.registerListener(rotL, rs, android.hardware.SensorManager.SENSOR_DELAY_UI);
        h.removeCallbacks(fastFind);
        h.post(fastFind);
        acked.add(key);
        shell.setVisibility(View.GONE);
        findView.setVisibility(View.VISIBLE);
        stopFlash();
        renderFinder(System.currentTimeMillis());
    }

    void closeFinder() {
        findKey = null;
        dirScanning = false;
        h.removeCallbacks(fastFind);
        if (sm != null) sm.unregisterListener(rotL);
        findView.setVisibility(View.GONE);
        shell.setVisibility(View.VISIBLE);
    }

    // ───────────────────────── 탭 · 누적 기준 ─────────────────────────

    LinearLayout shell, basePane, logPane, roomList;
    ScrollView baseScroll, logScroll;
    TextView baseHead, baseRoomText, logText;
    Button btnSurvey;
    final Button[] pageBtns = new Button[3];
    final Button[] gradeBtns2 = new Button[4];
    int page = 0; // 0 감시, 1 기준, 2 기록

    void setPage(int p) {
        page = p;
        main.setVisibility(p == 0 ? View.VISIBLE : View.GONE);
        baseScroll.setVisibility(p == 1 ? View.VISIBLE : View.GONE);
        logScroll.setVisibility(p == 2 ? View.VISIBLE : View.GONE);
        for (int i = 0; i < 3; i++) styleBtn(pageBtns[i], i == p ? WHITE : NAVY2, i == p ? NAVY : WHITE);
        render(System.currentTimeMillis());
    }

    /** 반 하나의 기준: 잰 시각, 그때 본 신호 열쇠들, 와이파이 지문(다음 단계 자동 위치용) */
    static class RoomBase {
        long t;
        final HashSet<String> keys = new HashSet<>();
        final HashMap<String, Double> fp = new HashMap<>();
    }
    static final int ROOM_LEGACY = 0, ROOM_IGNORED = -1, ROOM_SCHOOL = -2;
    final HashMap<String, String> labels = new HashMap<>();   // 학교 장비에 붙인 이름 (주소 → 이름)
    final HashSet<String> sessionIgnored = new HashSet<>();   // 이번 시험에서만 무시

    void saveLabels() { prefs.edit().putString("labels", new org.json.JSONObject(labels).toString()).apply(); }

    /** 같은 공유기의 다른 주소(2.4/5GHz 등): 고유 주소이고 앞 5바이트가 같은 와이파이 */
    List<Dev> siblings(Dev d) {
        List<Dev> out = new ArrayList<>();
        if (!d.wifi || isRandomMac(d) || d.key.length() < 16) return out;
        String p = d.key.substring(0, 16).toLowerCase(Locale.ROOT);
        for (Dev o : devs.values())
            if (o != d && o.wifi && !isRandomMac(o) && o.key.toLowerCase(Locale.ROOT).startsWith(p)) out.add(o);
        return out;
    }

    /** '처리' 버튼: 학교 장비로 등록(계속 면제) / 이번만 무시 */
    void handle(Dev d) {
        String g = groupKey(d);
        List<Dev> grp = new ArrayList<>();
        for (Dev o : devs.values()) if (groupKey(o).equals(g)) grp.add(o);
        if (!grp.contains(d)) grp.add(d);
        new AlertDialog.Builder(this)
                .setTitle(d.name)
                .setItems(new String[]{
                        "학교 장비로 등록 (공유기·무선 영상 공유기 등, 앞으로 계속 경고 안 함)",
                        "이번 시험에서만 무시",
                        "취소"}, (dl, w) -> {
                    if (w == 0) {
                        // 휴대폰·이어폰 등 개인 기기는 블루투스 주소가 바뀌어 등록해도 소용없고, 학생 기기를 놓칠 위험만 생긴다
                        boolean personal = d.wifi ? (d.phoneLike && !d.direct) : (d.rank >= 2 || (d.typeShort == null && d.manu >= 0));
                        if (personal) new AlertDialog.Builder(this)
                                .setTitle("개인 기기로 보입니다")
                                .setMessage("휴대폰·이어폰·워치는 블루투스 주소가 15분쯤마다 바뀌어 학교 장비로 등록해도 곧 다시 뜹니다.\n'이번 시험에서만 무시'를 추천합니다.")
                                .setPositiveButton("이번만 무시", (a, b) -> { for (Dev o : grp) sessionIgnored.add(o.key); render(System.currentTimeMillis()); })
                                .setNeutralButton("그래도 등록", (a, b) -> askSchoolName(d, grp))
                                .setNegativeButton("취소", null).show();
                        else askSchoolName(d, grp);
                    }
                    else if (w == 1) {
                        for (Dev o : grp) sessionIgnored.add(o.key);
                        Rec r = log.get(g);
                        if (r != null) { r.ignored = true; saveLog(); }
                        if (d.key.equals(findKey)) closeFinder();
                        render(System.currentTimeMillis());
                    }
                }).show();
    }

    void askSchoolName(Dev d, List<Dev> grp) {
        android.widget.EditText et = new android.widget.EditText(this);
        et.setHint("예: 3층 공유기, 2-3 무선 영상 공유기");
        et.setSingleLine(true);
        et.setText(d.name);
        List<Dev> sib = siblings(d);
        new AlertDialog.Builder(this)
                .setTitle("학교 장비 이름")
                .setMessage(sib.isEmpty() ? "알아보기 쉬운 이름을 붙여 두세요." : "같은 공유기의 다른 주소 " + sib.size() + "개도 함께 등록합니다.")
                .setView(et)
                .setPositiveButton("등록", (dl, w) -> {
                    String nm = et.getText().toString().trim();
                    if (nm.isEmpty()) nm = d.name;
                    RoomBase sc = roomOf(ROOM_SCHOOL);
                    sc.t = System.currentTimeMillis();
                    List<Dev> all = new ArrayList<>(grp);
                    all.addAll(sib);
                    for (Dev o : all) { sc.keys.add(o.key); labels.put(o.key, nm); baseSeen.put(o.key, sc.t); }
                    rebuildBaseline(); saveRooms(); saveLabels();
                    Rec r = log.get(groupKey(d));
                    if (r != null) { r.ignored = true; saveLog(); }
                    toast("'" + nm + "' 학교 장비로 등록했습니다 (주소 " + all.size() + "개).");
                    if (d.key.equals(findKey)) closeFinder();
                    render(System.currentTimeMillis());
                })
                .setNegativeButton("취소", null).show();
    }

    /** 기준 탭: 학교 장비 목록 보기·빼기 */
    void showSchoolList() {
        RoomBase sc = rooms.get(ROOM_SCHOOL);
        if (sc == null || sc.keys.isEmpty()) { toast("등록된 학교 장비가 없습니다."); return; }
        // 이름별로 묶어 보여 준다
        java.util.TreeMap<String, List<String>> byName = new java.util.TreeMap<>();
        for (String k : sc.keys) {
            String nm = labels.containsKey(k) ? labels.get(k) : k;
            List<String> l = byName.get(nm);
            if (l == null) { l = new ArrayList<>(); byName.put(nm, l); }
            l.add(k);
        }
        final List<String> names = new ArrayList<>(byName.keySet());
        String[] items = new String[names.size()];
        for (int i = 0; i < names.size(); i++) items[i] = names.get(i) + "  (주소 " + byName.get(names.get(i)).size() + "개)";
        new AlertDialog.Builder(this).setTitle("학교 장비 (누르면 뺄 수 있음)")
                .setItems(items, (dl, w) -> new AlertDialog.Builder(this)
                        .setTitle(names.get(w))
                        .setMessage("학교 장비에서 뺄까요? 빼면 다시 경고 대상이 됩니다.")
                        .setPositiveButton("빼기", (d2, w2) -> {
                            for (String k : byName.get(names.get(w))) { sc.keys.remove(k); labels.remove(k); }
                            if (sc.keys.isEmpty()) rooms.remove(ROOM_SCHOOL);
                            rebuildBaseline(); saveRooms(); saveLabels();
                            render(System.currentTimeMillis());
                        })
                        .setNegativeButton("취소", null).show())
                .setNegativeButton("닫기", null).show();
    }
    static final long KEEP_MS = 30L * 24 * 3600 * 1000; // 30일 동안 안 잡히면 지움
    final java.util.TreeMap<Integer, RoomBase> rooms = new java.util.TreeMap<>();
    final HashMap<String, Long> baseSeen = new HashMap<>();
    boolean roomsDirty = false;

    void rebuildBaseline() {
        baseline.clear();
        for (RoomBase rb : rooms.values()) baseline.addAll(rb.keys);
    }

    RoomBase roomOf(int code) {
        RoomBase rb = rooms.get(code);
        if (rb == null) { rb = new RoomBase(); rooms.put(code, rb); }
        return rb;
    }

    void saveRooms() {
        try {
            org.json.JSONObject o = new org.json.JSONObject();
            for (Map.Entry<Integer, RoomBase> e : rooms.entrySet()) {
                org.json.JSONObject r = new org.json.JSONObject();
                r.put("t", e.getValue().t);
                r.put("k", new org.json.JSONArray(e.getValue().keys));
                org.json.JSONObject fp = new org.json.JSONObject();
                for (Map.Entry<String, Double> f : e.getValue().fp.entrySet()) fp.put(f.getKey(), f.getValue());
                r.put("fp", fp);
                o.put(String.valueOf(e.getKey()), r);
            }
            org.json.JSONObject s = new org.json.JSONObject();
            for (Map.Entry<String, Long> e : baseSeen.entrySet()) s.put(e.getKey(), e.getValue());
            prefs.edit().putString("rooms", o.toString()).putString("baseSeen", s.toString()).apply();
            roomsDirty = false;
        } catch (Exception ignored) { }
    }

    void loadRooms() {
        long now = System.currentTimeMillis();
        try {
            org.json.JSONObject s = new org.json.JSONObject(prefs.getString("baseSeen", "{}"));
            java.util.Iterator<String> it = s.keys();
            while (it.hasNext()) { String k = it.next(); baseSeen.put(k, s.getLong(k)); }
            String raw = prefs.getString("rooms", null);
            if (raw != null) {
                org.json.JSONObject o = new org.json.JSONObject(raw);
                java.util.Iterator<String> rk = o.keys();
                while (rk.hasNext()) {
                    String code = rk.next();
                    org.json.JSONObject r = o.getJSONObject(code);
                    RoomBase rb = roomOf(Integer.parseInt(code));
                    rb.t = r.optLong("t");
                    org.json.JSONArray ka = r.optJSONArray("k");
                    if (ka != null) for (int i = 0; i < ka.length(); i++) rb.keys.add(ka.getString(i));
                    org.json.JSONObject fp = r.optJSONObject("fp");
                    if (fp != null) { java.util.Iterator<String> fi = fp.keys(); while (fi.hasNext()) { String f = fi.next(); rb.fp.put(f, fp.getDouble(f)); } }
                }
            } else if (!baseline.isEmpty()) {
                // 예전 버전 기준은 '이전 기준'으로 옮긴다
                RoomBase rb = roomOf(ROOM_LEGACY);
                rb.t = now;
                rb.keys.addAll(baseline);
            }
        } catch (Exception ignored) { }
        // 30일 넘게 안 잡힌 항목 정리 (날짜가 없는 항목은 오늘부터 센다)
        for (Map.Entry<Integer, RoomBase> en : rooms.entrySet()) {
            if (en.getKey() == ROOM_SCHOOL) continue; // 사람이 확인한 학교 장비는 자동으로 지우지 않는다
            en.getValue().keys.removeIf(k -> { Long t = baseSeen.get(k); if (t == null) { baseSeen.put(k, now); return false; } return now - t > KEEP_MS; });
        }
        try {
            org.json.JSONObject lb = new org.json.JSONObject(prefs.getString("labels", "{}"));
            java.util.Iterator<String> li = lb.keys();
            while (li.hasNext()) { String k = li.next(); labels.put(k, lb.getString(k)); }
        } catch (Exception ignored) { }
        rooms.values().removeIf(rb -> rb.keys.isEmpty());
        rebuildBaseline();
        saveRooms();
    }

    // 기준 재기(누적): 순회는 반 앞 20초, 교실 고정은 교실 가운데 60초
    boolean surveying = false;
    long surveyEnd = 0, surveyStart = 0;
    int surveyCode = 0;

    void startSurvey(boolean replace) {
        if (!hasPerms()) { askPermissions(); return; }
        if (surveying || measuring) return;
        if (mode == MODE_WATCH) { toast("감시 중에는 기준을 잴 수 없습니다. 감시를 멈춘 뒤 재 주세요."); return; }
        surveyCode = loc();
        if (replace) { rooms.remove(surveyCode); rebuildBaseline(); }
        mbuf.clear();
        surveying = true;
        surveyStart = System.currentTimeMillis();
        surveyEnd = surveyStart + (patrol ? 20_000 : 60_000);
        stopBle(); startBle(); kickWifiScan();
        render(surveyStart);
    }

    void finishSurvey() {
        surveying = false;
        long now = System.currentTimeMillis();
        RoomBase rb = roomOf(surveyCode);
        rb.t = now;
        int n = 0;
        for (Map.Entry<String, ArrayList<Integer>> e : mbuf.entrySet()) {
            Dev d = devs.get(e.getKey());
            if (d == null || e.getValue().isEmpty()) continue;
            for (String k : new String[]{d.key, nameKey(d), vendorKey(d)}) {
                if (k == null) continue;
                rb.keys.add(k);
                baseSeen.put(k, now);
            }
            if (d.wifi) {
                ArrayList<Integer> l = e.getValue();
                Collections.sort(l);
                rb.fp.put(d.key, (double) l.get(l.size() / 2));
            }
            n++;
        }
        mbuf.clear();
        if (rb.keys.isEmpty()) rooms.remove(surveyCode);
        rebuildBaseline();
        saveRooms();
        buzz(new long[]{0, 80});
        toast(roomName(surveyCode) + " 기준 " + n + "개 저장" + (patrol ? ". 다음 반으로 넘어갑니다." : "."));
        if (patrol && grade != 9) setRoom(room + 1); else render(now);
    }

    void removeRoom(int code) {
        rooms.remove(code);
        rebuildBaseline();
        saveRooms();
        render(System.currentTimeMillis());
    }

    String roomLabel(int code) {
        return code == ROOM_LEGACY ? "이전 버전 기준" : code == ROOM_IGNORED ? "예전에 무시한 기기"
                : code == ROOM_SCHOOL ? "학교 장비 (확인됨)" : roomName(code);
    }

    static int realCount(java.util.Collection<String> keys) {
        int n = 0;
        for (String k : keys) if (k.startsWith("W:") || k.startsWith("B:")) n++;
        return n;
    }

    void renderBasePane(long now) {
        baseHead.setText("누적 기준: 신호 " + baseCount() + "개 · " + rooms.size() + "곳\n30일 동안 한 번도 안 잡힌 항목은 자동으로 지워집니다.");
        baseRoomText.setText(roomName(loc()) + (grade == 9 ? "" : patrol ? " 앞" : " 교실"));
        styleGradeButtons(gradeBtns2);
        if (surveying) {
            btnSurvey.setText("재는 중 " + Math.max(1, (surveyEnd - now + 999) / 1000) + "초 · 가만히 서 계세요");
            btnSurvey.setEnabled(false);
            styleBtn(btnSurvey, SKY, NAVY);
        } else {
            RoomBase cur = rooms.get(loc());
            btnSurvey.setText((cur == null ? "여기서 기준 재기" : "여기서 기준 더하기") + (patrol ? " (20초)" : " (1분)"));
            btnSurvey.setEnabled(true);
            styleBtn(btnSurvey, LIME, NAVY);
        }
        roomList.removeAllViews();
        if (rooms.isEmpty()) roomList.addView(label("아직 잰 곳이 없습니다. 위에서 학년·반을 고르고 '여기서 기준 재기'를 누르세요.", 14, DIM, false));
        java.text.SimpleDateFormat f = new java.text.SimpleDateFormat("M/d HH:mm", Locale.KOREA);
        for (Map.Entry<Integer, RoomBase> e : rooms.entrySet()) {
            final int code = e.getKey();
            LinearLayout r = new LinearLayout(this);
            r.setGravity(Gravity.CENTER_VERTICAL);
            r.setPadding(dp(12), dp(10), dp(12), dp(10));
            r.setBackground(round(NAVY2, 12));
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(-1, -2);
            lp.bottomMargin = dp(8);
            LinearLayout mid = new LinearLayout(this);
            mid.setOrientation(LinearLayout.VERTICAL);
            mid.addView(label(roomLabel(code), 17, WHITE, true));
            mid.addView(label("신호 " + realCount(e.getValue().keys) + "개 · " + f.format(new java.util.Date(e.getValue().t)), 13, DIM, false));
            r.addView(mid, new LinearLayout.LayoutParams(0, -2, 1));
            if (code > 0) {
                Button again = btn("다시 재기", NAVY, LIME);
                again.setTextSize(13);
                again.setOnClickListener(v -> { grade = code / 100; room = code % 100;
                    prefs.edit().putInt("grade", grade).putInt("room", room).apply(); startSurvey(true); });
                r.addView(again, new LinearLayout.LayoutParams(-2, dp(40)));
            }
            if (code == ROOM_SCHOOL) {
                Button see = btn("목록 보기", NAVY, LIME);
                see.setTextSize(13);
                see.setOnClickListener(v -> showSchoolList());
                r.addView(see, new LinearLayout.LayoutParams(-2, dp(40)));
            }
            if (code != ROOM_IGNORED && code != ROOM_SCHOOL) {
                Button mvB = btn("곳 바꾸기", NAVY, SKY);
                mvB.setTextSize(13);
                mvB.setOnClickListener(v -> askPlaceName(to -> moveRoom(code, to)));
                LinearLayout.LayoutParams mlp2 = new LinearLayout.LayoutParams(-2, dp(40));
                mlp2.leftMargin = dp(8);
                r.addView(mvB, mlp2);
            }
            Button del = btn("지우기", NAVY, ORANGE);
            del.setTextSize(13);
            del.setOnClickListener(v -> new AlertDialog.Builder(this)
                    .setTitle(roomLabel(code) + " 기준 지우기")
                    .setMessage("이 곳에서 잰 기준만 지웁니다. 다른 반에서도 잡힌 신호는 그대로 남습니다.")
                    .setPositiveButton("지우기", (d, w) -> removeRoom(code))
                    .setNegativeButton("취소", null).show());
            LinearLayout.LayoutParams dlp2 = new LinearLayout.LayoutParams(-2, dp(40));
            dlp2.leftMargin = dp(8);
            r.addView(del, dlp2);
            roomList.addView(r, lp);
        }
    }

    void saveCsv() {
        saveLog();
        Intent i = new Intent(Intent.ACTION_CREATE_DOCUMENT);
        i.addCategory(Intent.CATEGORY_OPENABLE);
        i.setType("text/csv");
        i.putExtra(Intent.EXTRA_TITLE, "시험감독기록_" + new java.text.SimpleDateFormat("yyyyMMdd_HHmm", Locale.KOREA).format(new java.util.Date()) + ".csv");
        try { startActivityForResult(i, REQ_SAVE); } catch (Exception e) { toast("파일 저장 화면을 열 수 없습니다."); }
    }

    void shareSummary() {
        Intent i = new Intent(Intent.ACTION_SEND);
        i.setType("text/plain");
        i.putExtra(Intent.EXTRA_SUBJECT, "시험감독 레이더 기록");
        i.putExtra(Intent.EXTRA_TEXT, summary());
        startActivity(Intent.createChooser(i, "기록 보내기"));
    }

    double findShow = Double.NaN;                      // 찾기 화면에 보여 주는 다듬은 세기
    final ArrayList<double[]> findTrack = new ArrayList<>(); // [시각, 다듬은 세기]

    /** 최근 2초(모자라면 4초) 원본 값 중 상위 20% 지점. 반사로 순간 약해지는 값을 버린다. */
    static double stableRssi(Dev d, long now) {
        ArrayList<Integer> v = new ArrayList<>();
        for (long[] s : d.raw) if (now - s[0] <= 2000) v.add((int) s[1]);
        if (v.size() < 3) { v.clear(); for (long[] s : d.raw) if (now - s[0] <= 4000) v.add((int) s[1]); }
        if (v.isEmpty()) return d.ema;
        Collections.sort(v);
        return v.get(Math.min(v.size() - 1, (int) Math.floor(v.size() * 0.8)));
    }

    double avgTrack(long now, long fromAgo, long toAgo) {
        double s = 0; int n = 0;
        for (double[] p : findTrack) { long age = now - (long) p[0]; if (age >= fromAgo && age <= toAgo) { s += p[1]; n++; } }
        return n == 0 ? Double.NaN : s / n;
    }

    static double metersOf(Dev d, double rssi) {
        double ref = d.wifi ? -40 : (d.txPower >= -30 && d.txPower <= 20 ? d.txPower - 41 : -59);
        double n = d.wifi ? 2.7 : 2.2;
        return Math.max(0.1, Math.min(30, Math.pow(10, (ref - rssi) / (10 * n))));
    }

    void renderFinder(long now) {
        Dev d = devs.get(findKey);
        if (d == null) { closeFinder(); return; }
        findName.setText(d.name + "\n신호 점수 (0~100, 클수록 가깝습니다)");
        long ago = (now - d.lastSeen) / 1000;
        // 블루투스는 흔들림이 커서 '최근 2초 원본 값 중 위쪽(상위 20%)'을 쓰고, 화면 값은 한 번 더 천천히 따라가게 한다
        double st = d.wifi ? d.ema : stableRssi(d, now);
        findShow = Double.isNaN(findShow) ? st : findShow + Math.max(-3, Math.min(3, (st - findShow) * 0.35));
        findTrack.add(new double[]{now, findShow});
        while (!findTrack.isEmpty() && now - findTrack.get(0)[0] > 6000) findTrack.remove(0);
        double m = metersOf(d, findShow);
        if (isGone(d, now)) {
            findDist.setText("—");
            findDist.setTextColor(DIM);
            findTrend.setText("최근 미수신 · 마지막으로 잡힌 지 " + ago + "초\n기기를 껐거나, 가려졌거나, 잠깐 안 잡힌 것일 수 있습니다.");
            findTrend.setTextColor(DIM);
        } else {
            int sc = score(findShow);
            findDist.setText(String.valueOf(sc));
            findDist.setTextColor(sc >= 80 ? ORANGE : sc >= 55 ? LIME : WHITE);
            String dist = "거리 추정 " + fineDist(m) + "  ·  ";
            int dir;
            if (d.wifi) {
                // 와이파이는 새 검색 값 두 개로만 비교한다(그 사이엔 '기다리는 중')
                int hs = d.hist.size();
                if (hs < 2 || now - d.lastSeen > 15_000) dir = 9;
                else { double df = d.hist.get(hs - 1)[1] - d.hist.get(hs - 2)[1]; dir = df >= 2 ? 1 : df <= -2 ? -1 : 0; }
            } else {
                // 블루투스: 최근 1초 평균과 2~3초 전 평균을 비교(2dB 넘게 차이 날 때만 방향 표시)
                double recent = avgTrack(now, 0, 1000), before = avgTrack(now, 2000, 3000);
                if (Double.isNaN(recent) || Double.isNaN(before)) dir = 0;
                else { double df = recent - before; dir = df >= 2 ? 1 : df <= -2 ? -1 : 0; }
            }
            if (dir == 1) { findTrend.setText(dist + "▲ 가까워지는 중"); findTrend.setTextColor(LIME); }
            else if (dir == -1) { findTrend.setText(dist + "▼ 멀어지는 중"); findTrend.setTextColor(ORANGE); }
            else if (dir == 9) { findTrend.setText(dist + "새 값 기다리는 중"); findTrend.setTextColor(DIM); }
            else { findTrend.setText(dist + "● 그대로"); findTrend.setTextColor(WHITE); }
        }
        if (btnDir != null) {
            if (dirScanning) {
                btnDir.setText("도는 중 " + Math.max(1, (dirEnd - now + 999) / 1000) + "초 · 천천히 한 바퀴");
                btnDir.setEnabled(false);
            } else {
                btnDir.setText(Double.isNaN(prevAz) ? "방향 찾기 (제자리에서 한 바퀴, 12초)" : "한 번 더 돌기 (확인용)");
                btnDir.setEnabled(true);
            }
            btnDir.setVisibility(d.wifi ? View.GONE : View.VISIBLE);
        }
        findRaw.setText(peakLine(d) + zone(m) +" · 신호 " + Math.round(d.ema) + " dBm · " + d.kind
                + "\n태블릿을 들고 천천히 걸어 보세요. 숫자보다 ▲▼ 방향을 믿는 게 정확합니다."
                + (d.wifi ? (Build.VERSION.SDK_INT == 28 || wifiThrottled
                        ? "\n와이파이는 30초마다 갱신됩니다. 블루투스보다 천천히 움직이세요."
                        : "\n와이파이는 4초마다 갱신되어 조금 늦게 반응합니다.") : ""));
        // 막대: -95(0%) ~ -35(100%)
        double pct = Math.max(0.03, Math.min(1, ((Double.isNaN(findShow) ? d.ema : findShow) + 95) / 60.0));
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

        // ── 겉틀: 제목 + 업데이트 + [감시|기준|기록] 탭 + 각 화면 ──
        shell = new LinearLayout(this);
        shell.setOrientation(LinearLayout.VERTICAL);
        int g = dp(18);
        shell.setPadding(g, g, g, g);
        main = new LinearLayout(this);   // 감시 화면
        main.setOrientation(LinearLayout.VERTICAL);

        LinearLayout top = new LinearLayout(this);
        top.setGravity(Gravity.CENTER_VERTICAL);
        top.addView(label("시험감독 레이더", 22, WHITE, true));
        TextView beta = label("베타", 12, NAVY, true);
        beta.setBackground(round(LIME, 8));
        beta.setPadding(dp(8), dp(2), dp(8), dp(2));
        LinearLayout.LayoutParams blp = new LinearLayout.LayoutParams(-2, -2);
        blp.leftMargin = dp(8);
        top.addView(beta, blp);
        TextView ver = label("v" + myName(), 13, SKY, false);
        ver.setPadding(dp(10), dp(8), dp(4), dp(8));
        ver.setOnClickListener(v -> checkUpdate(true));
        top.addView(ver);
        top.addView(new View(this), new LinearLayout.LayoutParams(0, 1, 1));
        TextView help = label("사용법", 14, SKY, true);
        help.setPadding(dp(10), dp(8), dp(4), dp(8));
        help.setOnClickListener(v -> showHelp());
        top.addView(help);
        shell.addView(top);

        btnUpdate = btn("", SKY, NAVY);
        btnUpdate.setVisibility(View.GONE);
        btnUpdate.setOnClickListener(v -> doUpdate());
        LinearLayout.LayoutParams ulp = new LinearLayout.LayoutParams(-1, dp(50));
        ulp.topMargin = dp(10);
        shell.addView(btnUpdate, ulp);

        LinearLayout pt = hrow(dp(12));
        for (int pi = 0; pi < 3; pi++) {
            final int pp = pi;
            pageBtns[pi] = btn(new String[]{"감시", "기준", "기록"}[pi], NAVY2, WHITE);
            pageBtns[pi].setTextSize(17);
            pageBtns[pi].setOnClickListener(v -> setPage(pp));
            pt.addView(pageBtns[pi], weighted(dp(52), pi == 0 ? 0 : dp(8)));
        }
        shell.addView(pt);

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
        btnClearAll = btn("지금 뜬 것 모두 이번만 무시", NAVY, WHITE);
        btnClearAll.setTextSize(14);
        LinearLayout.LayoutParams calp2 = new LinearLayout.LayoutParams(-1, dp(44));
        calp2.topMargin = dp(8);
        btnClearAll.setLayoutParams(calp2);
        btnClearAll.setOnClickListener(v -> new AlertDialog.Builder(this)
                .setTitle("모두 이번만 무시")
                .setMessage("지금 새 신호와 확인 필요에 떠 있는 것을 이번 감시에서 모두 뺍니다. 학생이 이미 있으면 학생 기기도 빠지니 입실 전에만 쓰세요.")
                .setPositiveButton("모두 빼기", (d, w) -> {
                    long now = System.currentTimeMillis();
                    int n = 0;
                    for (Dev o : devs.values()) if (rawAlert(o, now)) { sessionIgnored.add(o.key); n++; }
                    stopFlash();
                    toast(n + "개를 이번 감시에서 뺐습니다.");
                    render(now);
                })
                .setNegativeButton("취소", null).show());
        statusCard.addView(btnClearAll);
        LinearLayout.LayoutParams sclp = new LinearLayout.LayoutParams(-1, -2);
        sclp.topMargin = dp(14);
        main.addView(statusCard, sclp);

        LinearLayout r1 = hrow(dp(12));
        btnBase = btn("", NAVY2, WHITE); // (예전 '기준 잡기' 버튼: 이제 기준 탭으로 옮김, 화면에는 안 붙임)
        btnWatch = btn("감시 시작", LIME, NAVY);
        btnWatch.setOnClickListener(v -> toggleWatch());
        r1.addView(btnWatch, weighted(dp(60), 0));
        main.addView(r1);

        // ── 기준 화면 ──
        basePane = new LinearLayout(this);
        basePane.setOrientation(LinearLayout.VERTICAL);
        baseHead = label("", 15, WHITE, false);
        baseHead.setPadding(0, dp(14), 0, dp(4));
        basePane.addView(baseHead);
        LinearLayout sbox = new LinearLayout(this);
        sbox.setOrientation(LinearLayout.VERTICAL);
        sbox.setPadding(dp(14), dp(12), dp(14), dp(12));
        sbox.setBackground(round(NAVY2, 14, LIME));
        LinearLayout sr = new LinearLayout(this);
        sr.setGravity(Gravity.CENTER_VERTICAL);
        Button sprev = btn("◀", NAVY, WHITE);
        sprev.setTextSize(20);
        sprev.setOnClickListener(v -> setRoom(room - 1));
        baseRoomText = label("", 26, WHITE, true);
        baseRoomText.setGravity(Gravity.CENTER);
        Button snext = btn("▶", NAVY, WHITE);
        snext.setTextSize(20);
        snext.setOnClickListener(v -> setRoom(room + 1));
        sr.addView(sprev, new LinearLayout.LayoutParams(dp(64), dp(64)));
        sr.addView(baseRoomText, new LinearLayout.LayoutParams(0, -2, 1));
        sr.addView(snext, new LinearLayout.LayoutParams(dp(64), dp(64)));
        sbox.addView(sr);
        LinearLayout sg = new LinearLayout(this);
        addGradeButtons(sg, gradeBtns2);
        LinearLayout.LayoutParams sglp = new LinearLayout.LayoutParams(-1, -2);
        sglp.topMargin = dp(8);
        sbox.addView(sg, sglp);
        btnSurvey = btn("", LIME, NAVY);
        btnSurvey.setTextSize(19);
        btnSurvey.setOnClickListener(v -> startSurvey(false));
        LinearLayout.LayoutParams svlp = new LinearLayout.LayoutParams(-1, dp(66));
        svlp.topMargin = dp(10);
        sbox.addView(btnSurvey, svlp);
        TextView sh = label("학생이 없을 때(방과 후 등) 반 앞마다 재면 기준에 계속 더해집니다. 여러 날 반복할수록 정확해집니다.", 13, DIM, false);
        sh.setPadding(0, dp(8), 0, 0);
        sbox.addView(sh);
        LinearLayout.LayoutParams sblp = new LinearLayout.LayoutParams(-1, -2);
        sblp.topMargin = dp(8);
        basePane.addView(sbox, sblp);
        TextView rlh = label("반별 기준", 16, LIME, true);
        rlh.setPadding(dp(4), dp(16), 0, dp(6));
        basePane.addView(rlh);
        roomList = new LinearLayout(this);
        roomList.setOrientation(LinearLayout.VERTICAL);
        basePane.addView(roomList);
        Button clearAll = btn("기준 전체 지우기", NAVY2, ORANGE);
        clearAll.setTextSize(14);
        clearAll.setOnClickListener(v -> new AlertDialog.Builder(this)
                .setTitle("기준 전체 지우기")
                .setMessage("쌓아 둔 모든 반의 기준과 무시 목록을 지웁니다. 되돌릴 수 없습니다.")
                .setPositiveButton("지우기", (d, w) -> { rooms.clear(); baseSeen.clear(); rebuildBaseline(); saveRooms(); render(System.currentTimeMillis()); })
                .setNegativeButton("취소", null).show());
        LinearLayout.LayoutParams calp = new LinearLayout.LayoutParams(-1, dp(46));
        calp.topMargin = dp(12);
        basePane.addView(clearAll, calp);
        TextView mvH = label("다른 태블릿과 자료 옮기기", 16, SKY, true);
        mvH.setPadding(dp(4), dp(18), 0, dp(6));
        basePane.addView(mvH);
        LinearLayout mv = hrow(0);
        Button sendB = btn("다른 태블릿으로 보내기", SKY, NAVY);
        sendB.setTextSize(14);
        sendB.setOnClickListener(v -> sendToOther());
        Button getB = btn("받은 자료 가져오기", NAVY2, SKY);
        getB.setTextSize(14);
        getB.setOnClickListener(v -> pickImport());
        mv.addView(sendB, weighted(dp(50), 0));
        mv.addView(getB, weighted(dp(50), dp(8)));
        basePane.addView(mv);
        TextView mvt = label("보내기를 누르고 'Quick Share'로 옆 태블릿을 고르면 됩니다. 받은 태블릿에서 알림을 눌러 파일을 열거나 '받은 자료 가져오기'를 누르세요. 반별 기준·기록·설정이 한 번에 옮겨집니다.", 13, DIM, false);
        mvt.setPadding(dp(4), dp(6), dp(4), 0);
        basePane.addView(mvt);
        TextView setH = label("설정", 16, SKY, true);
        setH.setPadding(dp(4), dp(18), 0, dp(6));
        basePane.addView(setH);

        LinearLayout r2 = hrow(dp(10));
        btnRange = btn("", NAVY2, WHITE);
        btnRange.setTextSize(14);
        btnRange.setOnClickListener(v -> { wide = !wide; prefs.edit().putBoolean("wide", wide).apply(); });
        btnSound = btn("", NAVY2, WHITE);
        btnSound.setTextSize(14);
        btnSound.setOnClickListener(v -> { sound = !sound; prefs.edit().putBoolean("sound", sound).apply(); });
        r2.addView(btnRange, weighted(dp(46), 0));
        r2.addView(btnSound, weighted(dp(46), dp(10)));
        basePane.addView(r2);

        btnPatrol = btn("", NAVY2, WHITE);
        btnPatrol.setTextSize(14);
        btnPatrol.setOnClickListener(v -> togglePatrol());
        LinearLayout.LayoutParams plp = new LinearLayout.LayoutParams(-1, dp(46));
        plp.topMargin = dp(10);
        basePane.addView(btnPatrol, plp);

        // 순회 칸: [◀] 3반 앞 [다음 교실 ▶] + 가장 센 새 신호
        patrolBox = new LinearLayout(this);
        patrolBox.setOrientation(LinearLayout.VERTICAL);
        patrolBox.setPadding(dp(14), dp(12), dp(14), dp(12));
        patrolBox.setBackground(round(NAVY2, 14, SKY));
        LinearLayout rr = new LinearLayout(this);
        rr.setGravity(Gravity.CENTER_VERTICAL);
        Button prev = btn("◀", NAVY, WHITE);
        prev.setTextSize(20);
        prev.setOnClickListener(v -> setRoom(room - 1));
        roomText = label("", 26, WHITE, true);
        roomText.setGravity(Gravity.CENTER);
        roomText.setOnLongClickListener(v -> { setRoom(1); return true; });
        Button next = btn("▶", NAVY, WHITE);
        next.setTextSize(20);
        next.setOnClickListener(v -> setRoom(room + 1));
        rr.addView(prev, new LinearLayout.LayoutParams(dp(64), dp(64)));
        rr.addView(roomText, new LinearLayout.LayoutParams(0, -2, 1));
        rr.addView(next, new LinearLayout.LayoutParams(dp(64), dp(64)));
        patrolBox.addView(rr);
        LinearLayout gr = new LinearLayout(this);
        addGradeButtons(gr, gradeBtns);
        LinearLayout.LayoutParams glp = new LinearLayout.LayoutParams(-1, -2);
        glp.topMargin = dp(8);
        patrolBox.addView(gr, glp);
        btnMeasure = btn("", SKY, NAVY);
        btnMeasure.setTextSize(20);
        btnMeasure.setOnClickListener(v -> startMeasure());
        LinearLayout.LayoutParams mlp = new LinearLayout.LayoutParams(-1, dp(68));
        mlp.topMargin = dp(10);
        patrolBox.addView(btnMeasure, mlp);
        topText = label("", 17, WHITE, true);
        topText.setPadding(0, dp(10), 0, 0);
        patrolBox.addView(topText);
        TextView ph = label("반을 옮길 때 ▶ 또는 '여기서 재기'를 눌러야 반별 판정이 맞습니다.", 12, DIM, false);
        ph.setPadding(0, dp(6), 0, 0);
        patrolBox.addView(ph);
        LinearLayout.LayoutParams pblp = new LinearLayout.LayoutParams(-1, -2);
        pblp.topMargin = dp(12);
        main.addView(patrolBox, pblp);

        // ── 기록 화면 ──
        logPane = new LinearLayout(this);
        logPane.setOrientation(LinearLayout.VERTICAL);
        LinearLayout lb = hrow(dp(14));
        Button saveB = btn("파일로 저장 (엑셀)", LIME, NAVY);
        saveB.setTextSize(15);
        saveB.setOnClickListener(v -> saveCsv());
        Button shareB = btn("공유", NAVY2, WHITE);
        shareB.setTextSize(15);
        shareB.setOnClickListener(v -> shareSummary());
        lb.addView(saveB, weighted(dp(50), 0));
        lb.addView(shareB, weighted(dp(50), dp(8)));
        logPane.addView(lb);
        logText = label("", 16, WHITE, false);
        logText.setPadding(dp(4), dp(14), dp(4), dp(14));
        logPane.addView(logText);
        Button newLog = btn("기록 비우고 새로 시작", NAVY2, ORANGE);
        newLog.setTextSize(14);
        newLog.setOnClickListener(v -> new AlertDialog.Builder(this)
                .setTitle("기록 비우기")
                .setMessage("지금까지의 감시 기록을 지웁니다. 필요하면 먼저 파일로 저장하세요.")
                .setPositiveButton("비우기", (d, w) -> { log.clear(); watchStart = mode == MODE_WATCH ? System.currentTimeMillis() : 0; saveLog(); render(System.currentTimeMillis()); })
                .setNegativeButton("취소", null).show());
        logPane.addView(newLog, new LinearLayout.LayoutParams(-1, dp(46)));

        tip = label("", 14, NAVY, false);
        tip.setBackground(round(LIME, 12));
        tip.setPadding(dp(14), dp(10), dp(14), dp(10));
        LinearLayout.LayoutParams tlp = new LinearLayout.LayoutParams(-1, -2);
        tlp.topMargin = dp(12);
        main.addView(tip, tlp);

        // 목록 탭: 와이파이 / 블루투스 / 전체
        LinearLayout tabs = hrow(dp(18));
        tabWifi = btn("", NAVY2, WHITE);
        tabBle = btn("", NAVY2, WHITE);
        tabAll = btn("전체", NAVY2, WHITE);
        for (Button tb : new Button[]{tabWifi, tabBle, tabAll}) tb.setTextSize(15);
        tabWifi.setOnClickListener(v -> { tab = 1; render(System.currentTimeMillis()); });
        tabBle.setOnClickListener(v -> { tab = 2; render(System.currentTimeMillis()); });
        tabAll.setOnClickListener(v -> { tab = 0; render(System.currentTimeMillis()); });
        tabs.addView(tabWifi, weighted(dp(48), 0));
        tabs.addView(tabBle, weighted(dp(48), dp(8)));
        tabs.addView(tabAll, weighted(dp(48), dp(8)));
        main.addView(tabs);
        View gapV = new View(this);
        main.addView(gapV, new LinearLayout.LayoutParams(1, dp(10)));
        ScrollView sv = new ScrollView(this);
        list = new LinearLayout(this);
        list.setOrientation(LinearLayout.VERTICAL);
        sv.addView(list);
        main.addView(sv, new LinearLayout.LayoutParams(-1, 0, 1));

        baseInfo = label("", 13, DIM, false);
        baseInfo.setPadding(0, dp(8), 0, 0);
        main.addView(baseInfo);
        shell.addView(main, new LinearLayout.LayoutParams(-1, 0, 1));
        baseScroll = new ScrollView(this);
        baseScroll.addView(basePane);
        shell.addView(baseScroll, new LinearLayout.LayoutParams(-1, 0, 1));
        logScroll = new ScrollView(this);
        logScroll.addView(logPane);
        shell.addView(logScroll, new LinearLayout.LayoutParams(-1, 0, 1));
        root.addView(shell, new FrameLayout.LayoutParams(-1, -1));
        setPage(page);

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
        arrow = label("↑", 110, ORANGE, true);
        arrow.setGravity(Gravity.CENTER);
        arrow.setVisibility(View.INVISIBLE);
        findView.addView(arrow, new LinearLayout.LayoutParams(-1, dp(150)));
        dirText = label("", 14, WHITE, false);
        dirText.setGravity(Gravity.CENTER);
        findView.addView(dirText, new LinearLayout.LayoutParams(-1, -2));
        btnDir = btn("방향 찾기 (제자리에서 한 바퀴, 12초)", SKY, NAVY);
        btnDir.setOnClickListener(v -> startDir());
        LinearLayout.LayoutParams dlp = new LinearLayout.LayoutParams(-1, dp(54));
        dlp.topMargin = dp(8);
        findView.addView(btnDir, dlp);
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
        Button ign = btn("괜찮은 기기 (학교 장비 / 이번만 무시)", NAVY2, WHITE);
        ign.setOnClickListener(v -> { Dev d = devs.get(findKey); if (d != null) handle(d); });
        findView.addView(ign, new LinearLayout.LayoutParams(-1, dp(52)));
        root.addView(findView, new FrameLayout.LayoutParams(-1, -1));

        setContentView(root);
    }

    void showHelp() {
        new AlertDialog.Builder(this)
                .setTitle("사용법")
                .setMessage("① 학생 입실 전, 빈 교실에서 '기준 잡기'를 누릅니다(1분). 학교 공유기, 옆 반 기기, 감독관 폰이 '원래 있던 것'으로 저장됩니다.\n\n"
                        + "② 끝나면 저절로 감시가 시작됩니다. 새 신호가 가까이 나타나면 화면이 깜빡이고 진동합니다.\n\n"
                        + "③ 목록에서 항목을 누르면 거리 화면이 나옵니다. 태블릿을 들고 걸으며 ▲(가까워짐)가 뜨는 쪽으로 가세요.\n\n"
                        + "④ 괜찮은 기기면 '무시'를 누르면 다시 경고하지 않습니다.\n\n"
                        + "복도 순회 모드\n"
                        + "· 태블릿 한 대로 한 층을 살핍니다. 기준은 입실 전 복도를 끝까지 한 번 걸으며 잡고, 다 걸었으면 '기준 잡기 끝'을 누릅니다.\n"
                        + "· '1분 기준'은 교실 고정 모드(태블릿을 한 교실에 두는 방식)에서만 씁니다.\n"
                        + "· 교실 문 앞(가운데)에 멈춰 '여기서 재기'를 누르고 가만히 서 있습니다. 신호가 충분히 모이면(5~15초) 다음 반으로 자동으로 넘어갑니다.\n"
                        + "· '무시'를 누른 기기는 목록에서 사라지고 다시 경고하지 않습니다(기록에는 '무시'로 남습니다). 새 기준을 잡으면 초기화됩니다.\n"
                        + "· 층을 옮기면 위의 학년 버튼을 누르세요(1반부터 다시 셉니다).\n"
                        + "· 새 신호마다 '○학년 ○반 쪽 · 잰 곳 중 뚜렷하게 강함 / 조금 강함 / 판정 불가'가 뜹니다. 잰 곳끼리 비교한 것이라, 기기가 잰 곳 밖(위층·복도)에 있을 수도 있습니다.\n"
                        + "· 목록은 세 칸입니다: 새 신호(경고) / 확인 필요(경고 아님, 접혀 있음) / 사라진 신호(잡혔다가 끊김).\n"
                        + "· 아래 상태줄이 '정상'일 때만 '새 신호 없음'을 믿으세요. 전원·무선을 끈 기기, 비행기 모드 폰은 잡히지 않습니다.\n"
                        + "· 매번 같은 높이·같은 방향으로 들고 재야 정확합니다(몸이 신호를 가립니다).\n"
                        + "· 한 바퀴 돈 뒤 가장 센 교실 감독에게 알려 주세요.\n\n"
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
