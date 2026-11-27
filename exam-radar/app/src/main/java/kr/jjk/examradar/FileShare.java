package kr.jjk.examradar;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.ParcelFileDescriptor;
import android.provider.OpenableColumns;

import java.io.File;
import java.io.FileNotFoundException;

/** 앱 캐시의 share 폴더 파일을 다른 앱(Quick Share 등)에 읽기 전용으로 건네는 작은 제공자 */
public class FileShare extends ContentProvider {
    static final String AUTH = "kr.jjk.examradar.files";

    File fileOf(Uri u) throws FileNotFoundException {
        File dir = new File(getContext().getCacheDir(), "share");
        File f = new File(dir, u.getLastPathSegment());
        try {
            if (!f.getCanonicalPath().startsWith(dir.getCanonicalPath() + File.separator)) throw new FileNotFoundException();
        } catch (java.io.IOException e) { throw new FileNotFoundException(); }
        if (!f.exists()) throw new FileNotFoundException();
        return f;
    }

    @Override public boolean onCreate() { return true; }

    @Override public ParcelFileDescriptor openFile(Uri uri, String mode) throws FileNotFoundException {
        return ParcelFileDescriptor.open(fileOf(uri), ParcelFileDescriptor.MODE_READ_ONLY);
    }

    @Override public String getType(Uri uri) { return "application/json"; }

    @Override public Cursor query(Uri uri, String[] proj, String sel, String[] args, String sort) {
        try {
            File f = fileOf(uri);
            MatrixCursor c = new MatrixCursor(new String[]{OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE});
            c.addRow(new Object[]{f.getName(), f.length()});
            return c;
        } catch (FileNotFoundException e) { return null; }
    }

    @Override public Uri insert(Uri uri, ContentValues v) { return null; }
    @Override public int delete(Uri uri, String s, String[] a) { return 0; }
    @Override public int update(Uri uri, ContentValues v, String s, String[] a) { return 0; }
}
