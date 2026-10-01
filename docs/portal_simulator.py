import os
import sys
import datetime
from dataclasses import dataclass
from enum import Enum, auto

from PySide6.QtCore import QPoint, QRect, QSize, Qt, QDateTime, QTimer, Signal, QPropertyAnimation, QEasingCurve
from PySide6.QtGui import QColor, QFont, QPainter, QPen
from PySide6.QtWidgets import (
    QAbstractItemView, QApplication, QCheckBox, QComboBox, QFrame, QGridLayout, QHBoxLayout,
    QLabel, QListWidget, QListWidgetItem, QMainWindow, QPushButton, QSlider,
    QVBoxLayout, QWidget, QMenu, QLineEdit, QSizePolicy, QListView, QGraphicsDropShadowEffect, QGraphicsOpacityEffect,
    QTreeWidget, QTreeWidgetItem, QStackedWidget, QHeaderView, QSplitter, QTextEdit, QTableWidget, QTableWidgetItem,
    QTabWidget, QScrollArea, QProgressBar, QGroupBox, QButtonGroup, QRadioButton, QTextBrowser
)


# ---------------------------------------------------------------------------
# Theme
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class Theme:
    name: str
    desktop: str
    taskbar: str
    taskbar_border: str
    window: str
    window_border: str
    title_active: str
    title_inactive: str
    title_text: str
    text: str
    secondary_text: str
    button: str
    button_hover: str
    button_pressed: str
    accent: str
    panel: str
    desktop_text: str


THEMES = {
    "Classic": Theme(
        "Classic", "#15223a", "#101624", "#42618e", "#edf2f8",
         "#58749d", "#3d6fa8", "#263a59", "#ffffff", "#687783",
        "#536275", "#d8e2ee", "#e8f0f8", "#b9c9da", "#4c8ed8", "#ffffff", "#ffffff"
    ),
    "Modern": Theme(
        "Modern", "#1e5f88", "#132e40", "#6da5c5", "#eef7fb",
        "#5b8da8", "#3e7fa5", "#426174", "#ffffff", "#687783",
        "#526872", "#dbeaf1", "#edf7fb", "#c9e0eb", "#48a8e6", "#ffffff", "#ffffff"
    ),
    "Traditional": Theme(
        "Traditional", "#211b1c", "#151314", "#5c4c45", "#292425",
        "#5b4a42", "#70574c", "#382f2c", "#f2e9e2", "#687783",
        "#a99a91", "#3b3431", "#4a403c", "#2c2725", "#c8794b", "#322c2a", "#f2e9e2"
    ),
    "Dark": Theme(
        "Dark", "#161616", "#0d0d0d", "#383838", "#242424", "#4a4a4a",
        "#3d3d3d", "#2b2b2b", "#f4f4f4", "#eeeeee", "#687783",
        "#333333", "#404040", "#292929", "#6ea8ff", "#2c2c2c", "#ffffff"
    ),
    "Light": Theme(
        "Light", "#dfe6ee", "#c8d0da", "#9ba9b8", "#f7f9fb", "#aab6c2",
        "#6f879e", "#9aa9b7", "#ffffff", "#1d2730", "#687783",
        "#e4e9ee", "#eef2f5", "#d6dde4", "#4d86c5", "#ffffff", "#1d2730"
    ),
}


# ---------------------------------------------------------------------------
# Window state & Policies
# ---------------------------------------------------------------------------

class WindowState(Enum):
    NORMAL = auto()
    MINIMIZED = auto()
    MAXIMIZED = auto()
    CLOSED = auto()


class TitleBarPosition(Enum):
    TOP = auto()
    BOTTOM = auto()
    NONE = auto()


@dataclass
class WindowLayoutPolicy:
    title_height: int = 34
    border_width: int = 1
    resize_margin: int = 8
    min_width: int = 260
    min_height: int = 160
    title_position: TitleBarPosition = TitleBarPosition.TOP
    show_minimize: bool = True
    show_maximize: bool = True
    show_close: bool = True
    show_icon: bool = True


@dataclass
class WindowBehaviorPolicy:
    allow_drag: bool = True
    allow_resize: bool = True
    allow_maximize: bool = True
    allow_minimize: bool = True
    allow_close: bool = True


@dataclass
class WindowGeometry:
    x: int
    y: int
    width: int
    height: int
    previous: QRect | None = None


# ---------------------------------------------------------------------------
# Explorer View
# ---------------------------------------------------------------------------

class ExplorerView(QListWidget):
    itemActionTriggered = Signal(object)

    def __init__(self, portal, parent=None, is_desktop=False):
        super().__init__(parent)
        self.portal = portal
        self.is_desktop = is_desktop
        self.setViewMode(QListWidget.IconMode)
        self.setMovement(QListWidget.Free)
        self.setSelectionMode(QAbstractItemView.ExtendedSelection)
        self.setSpacing(12)
        self.setGridSize(QSize(84, 84))
        self.setWordWrap(True)
        self.setIconSize(QSize(48, 48))
        self.setContextMenuPolicy(Qt.CustomContextMenu)
        self.customContextMenuRequested.connect(self.show_context_menu)
        self.itemDoubleClicked.connect(self.item_double_clicked)
        
        if is_desktop:
            self.setObjectName("desktopExplorer")
        else:
            self.setObjectName("windowExplorer")
            
        self.setCursor(Qt.ArrowCursor)

    def add_item(self, text, icon, data):
        item = QListWidgetItem(f"{icon}\n{text}")
        item.setTextAlignment(Qt.AlignCenter)
        item.setData(Qt.UserRole, data)
        self.addItem(item)

    def show_context_menu(self, pos):
        item = self.itemAt(pos)
        menu = QMenu(self)
        menu.setStyleSheet(f"""
            QMenu {{
                background: {self.portal.theme.window};
                border: 1px solid {self.portal.theme.window_border};
                color: {self.portal.theme.text};
            }}
            QMenu::item {{
                padding: 6px 24px 6px 24px;
            }}
            QMenu::item:selected {{
                background: {self.portal.theme.button_hover};
            }}
        """)
        
        if item:
            data = item.data(Qt.UserRole)
            if data:
                action = menu.addAction("Open")
                action.triggered.connect(lambda: self.itemActionTriggered.emit(data))
                if data.get("type") in ("file", "folder"):
                    menu.addAction("Properties")
            
            menu.addSeparator()
            menu.addAction("Delete")
        else:
            menu.addAction("Refresh")
            if self.is_desktop:
                menu.addAction("Personalize").triggered.connect(lambda: self.portal.launch("settings"))
        
        menu.exec(self.mapToGlobal(pos))

    def item_double_clicked(self, item):
        data = item.data(Qt.UserRole)
        if data:
            self.itemActionTriggered.emit(data)

class DetailsView(QTreeWidget):
    itemActionTriggered = Signal(object)

    def __init__(self, portal, parent=None):
        super().__init__(parent)
        self.portal = portal
        self.setObjectName("detailsView")
        self.setColumnCount(4)
        self.setHeaderLabels(["Name", "Type", "Size", "Date Modified"])
        self.setSelectionMode(QAbstractItemView.ExtendedSelection)
        self.setRootIsDecorated(False)
        self.setContextMenuPolicy(Qt.CustomContextMenu)
        self.customContextMenuRequested.connect(self.show_context_menu)
        self.itemDoubleClicked.connect(self.item_double_clicked)
        self.header().setContextMenuPolicy(Qt.CustomContextMenu)
        self.header().customContextMenuRequested.connect(self.show_header_menu)
        
        for col in self.portal.settings.get("hidden_columns", []):
            self.hideColumn(col)

    def add_item(self, name, icon, data, type_str, size_str, date_str):
        item = QTreeWidgetItem([f"{icon}  {name}", type_str, size_str, date_str])
        item.setData(0, Qt.UserRole, data)
        self.addTopLevelItem(item)

    def show_context_menu(self, pos):
        item = self.itemAt(pos)
        menu = QMenu(self)
        if item:
            data = item.data(0, Qt.UserRole)
            if data:
                action = menu.addAction("Open")
                action.triggered.connect(lambda: self.itemActionTriggered.emit(data))
                if data.get("type") in ("file", "folder"):
                    menu.addAction("Properties")
            menu.addSeparator()
            menu.addAction("Delete")
        else:
            menu.addAction("Refresh")
        menu.exec(self.mapToGlobal(pos))

    def show_header_menu(self, pos):
        menu = QMenu(self)
        for i in range(1, 4):
            col_name = self.headerItem().text(i)
            action = menu.addAction(col_name)
            action.setCheckable(True)
            action.setChecked(not self.isColumnHidden(i))
            action.toggled.connect(lambda checked, col=i: self.toggle_column(col, not checked))
        menu.exec(self.header().mapToGlobal(pos))

    def toggle_column(self, col, hidden):
        self.setColumnHidden(col, hidden)
        hidden_cols = self.portal.settings.get("hidden_columns", [])
        if hidden and col not in hidden_cols:
            hidden_cols.append(col)
        elif not hidden and col in hidden_cols:
            hidden_cols.remove(col)
        self.portal.update_setting("hidden_columns", hidden_cols)

    def item_double_clicked(self, item, column):
        data = item.data(0, Qt.UserRole)
        if data:
            self.itemActionTriggered.emit(data)

# ---------------------------------------------------------------------------
# Small application API
# ---------------------------------------------------------------------------

class PortalApplication:
    application_id = "application"
    display_name = "Application"
    icon = "?"
    default_size = (1920, 920)
    layout_policy: WindowLayoutPolicy = WindowLayoutPolicy()
    behavior_policy: WindowBehaviorPolicy = WindowBehaviorPolicy()

    def create_widget(self, portal):
        return QLabel("Empty application")


class HomeApplication(PortalApplication):
    application_id = "home"
    display_name = "Home"
    icon = "⌂"
    default_size = (1200, 720)

    def create_widget(self, portal):
        w = QWidget()
        layout = QVBoxLayout(w)
        title = QLabel("Portal Home")
        title.setObjectName("sectionTitle")
        layout.addWidget(title)
        layout.addWidget(QLabel(
            "A deliberately simple Qt Widgets environment for prototyping "
            "desktop-style interfaces."
        ))
        info = QLabel(
            "This window is only a visual test case. The simulator itself "
            "owns window state, focus and z-order."
        )
        info.setWordWrap(True)
        layout.addWidget(info)
        layout.addStretch()
        return w


class SettingsApplication(PortalApplication):
    application_id = "settings"
    display_name = "Settings"
    icon = "⚙"
    default_size = (560, 410)

    def create_widget(self, portal):
        w = QWidget()
        layout = QVBoxLayout(w)

        title = QLabel("Appearance")
        title.setObjectName("sectionTitle")
        layout.addWidget(title)

        row = QHBoxLayout()
        row.addWidget(QLabel("Theme"))
        combo = QComboBox()
        combo.addItems(THEMES.keys())
        combo.setCurrentText(portal.theme.name)
        combo.currentTextChanged.connect(portal.set_theme)
        row.addWidget(combo, 1)
        layout.addLayout(row)

        row = QHBoxLayout()
        row.addWidget(QLabel("Font Size (pt)"))
        slider = QSlider(Qt.Horizontal)
        slider.setRange(8, 16)
        slider.setValue(portal.settings.get("font_size", 10))
        slider.valueChanged.connect(lambda v: portal.update_setting("font_size", v))
        row.addWidget(slider, 1)
        layout.addLayout(row)

        checks = QVBoxLayout()
        
        c_icons = QCheckBox("Show desktop icons")
        c_icons.setChecked(portal.settings.get("show_desktop_icons", True))
        c_icons.toggled.connect(lambda v: portal.update_setting("show_desktop_icons", v))
        checks.addWidget(c_icons)
        
        c_shadows = QCheckBox("Enable window shadows")
        c_shadows.setChecked(portal.settings.get("window_shadows", False))
        c_shadows.toggled.connect(lambda v: portal.update_setting("window_shadows", v))
        checks.addWidget(c_shadows)
        
        c_taskbar = QCheckBox("Keep taskbar visible (uncheck for auto-hide)")
        c_taskbar.setChecked(portal.settings.get("auto_hide_taskbar", False) == False)
        c_taskbar.toggled.connect(lambda v: portal.update_setting("auto_hide_taskbar", not v))
        checks.addWidget(c_taskbar)
        
        c_clock24 = QCheckBox("Use 24-hour clock")
        c_clock24.setChecked(portal.settings.get("clock_24hr", True))
        c_clock24.toggled.connect(lambda v: portal.update_setting("clock_24hr", v))
        checks.addWidget(c_clock24)
        
        c_date = QCheckBox("Show date in taskbar")
        c_date.setChecked(portal.settings.get("show_date", False))
        c_date.toggled.connect(lambda v: portal.update_setting("show_date", v))
        checks.addWidget(c_date)
        
        layout.addLayout(checks)
        layout.addStretch()

        return w


class FileManagerWidget(QWidget):
    def __init__(self, portal, start_path):
        super().__init__()
        self.portal = portal
        self.current_path = os.path.abspath(start_path)
        
        layout = QVBoxLayout(self)
        
        toolbar = QHBoxLayout()
        self.up_button = QPushButton("↑ Up")
        self.up_button.clicked.connect(self.go_up)
        toolbar.addWidget(self.up_button)
        
        self.path_input = QLineEdit()
        self.path_input.returnPressed.connect(self.on_path_entered)
        toolbar.addWidget(self.path_input, 1)
        
        self.view_toggle = QPushButton("☷ View")
        self.view_toggle.clicked.connect(self.toggle_view)
        toolbar.addWidget(self.view_toggle)
        
        layout.addLayout(toolbar)

        self.stack = QStackedWidget(self)
        layout.addWidget(self.stack)

        self.explorer = ExplorerView(portal, self, is_desktop=False)
        self.explorer.itemActionTriggered.connect(self.handle_action)
        self.stack.addWidget(self.explorer)
        
        self.details = DetailsView(portal, self)
        self.details.itemActionTriggered.connect(self.handle_action)
        self.stack.addWidget(self.details)
        
        self.load_path(self.current_path)
        
    def on_path_entered(self):
        new_path = self.path_input.text()
        if os.path.exists(new_path):
            self.load_path(new_path)
        else:
            self.path_input.setText(self.current_path)
            
    def toggle_view(self):
        current = self.stack.currentIndex()
        self.stack.setCurrentIndex((current + 1) % 2)

    def load_path(self, path):
        self.current_path = os.path.abspath(path)
        self.path_input.setText(self.current_path)
        self.explorer.clear()
        self.details.clear()
        
        try:
            entries = os.listdir(self.current_path)
        except PermissionError:
            entries = []
            
        folders = []
        files = []
        for name in entries:
            full_path = os.path.join(self.current_path, name)
            if os.path.isdir(full_path):
                folders.append((name, full_path))
            else:
                files.append((name, full_path))
                
        folders.sort(key=lambda x: str.lower(x[0]))
        files.sort(key=lambda x: str.lower(x[0]))
        
        def format_time(ts):
            try:
                return datetime.datetime.fromtimestamp(ts).strftime("%m/%d/%Y %H:%M")
            except:
                return ""
                
        def format_size(sz):
            if sz < 1024: return f"{sz} B"
            elif sz < 1024*1024: return f"{sz//1024} KB"
            else: return f"{sz//(1024*1024)} MB"
        
        for name, fp in folders:
            stat = os.stat(fp) if os.path.exists(fp) else None
            date_str = format_time(stat.st_mtime) if stat else ""
            data = {"type": "folder", "path": fp}
            self.explorer.add_item(name, "📁", data)
            self.details.add_item(name, "📁", data, "File folder", "", date_str)
            
        for name, fp in files:
            stat = os.stat(fp) if os.path.exists(fp) else None
            date_str = format_time(stat.st_mtime) if stat else ""
            size_str = format_size(stat.st_size) if stat else ""
            ext = os.path.splitext(name)[1].upper().replace(".", "") + " File"
            data = {"type": "file", "path": fp}
            self.explorer.add_item(name, "📄", data)
            self.details.add_item(name, "📄", data, ext, size_str, date_str)
            
    def go_up(self):
        parent_path = os.path.dirname(self.current_path)
        if parent_path != self.current_path:
            self.load_path(parent_path)

    def handle_action(self, data):
        if data.get("type") == "folder":
            self.load_path(data["path"])
        elif data.get("type") == "app":
            self.portal.launch(data["id"])

class FileManagerApplication(PortalApplication):
    application_id = "files"
    display_name = "My Files"
    icon = "📁"
    default_size = (560, 390)

    def create_widget(self, portal):
        start_path = os.path.dirname(os.path.abspath(__file__))
        return FileManagerWidget(portal, start_path)


class AboutApplication(PortalApplication):
    application_id = "about"
    display_name = "About"
    icon = "ⓘ"
    default_size = (430, 300)

    def create_widget(self, portal):
        w = QWidget()
        layout = QVBoxLayout(w)
        title = QLabel("BSPT")
        title.setObjectName("sectionTitle")
        layout.addWidget(title)
        layout.addWidget(QLabel("PySide6 / Qt Widgets"))
        text = QLabel(
            "Single top-level Qt window.\n"
            "Internal windows are widgets.\n"
            "Theme, geometry and window state are deliberately separated."
        )
        text.setWordWrap(True)
        layout.addWidget(text)
        layout.addStretch()
        return w


class GridWidget(QWidget):
    """Widget displaying a 2D grid of empty black cells."""
    def __init__(self, portal, initial_rows=12, initial_cols=12):
        super().__init__()
        self.portal = portal
        self.rows = initial_rows
        self.cols = initial_cols

        layout = QVBoxLayout(self)
        layout.setContentsMargins(10, 10, 10, 10)
        layout.setSpacing(8)

        # Controls bar
        ctrl_layout = QHBoxLayout()
        ctrl_layout.setSpacing(8)

        lbl = QLabel("Grid Size:")
        ctrl_layout.addWidget(lbl)

        self.size_combo = QComboBox()
        self.size_combo.addItems(["8 x 8", "10 x 10", "12 x 12", "16 x 16", "20 x 20", "24 x 24", "32 x 32"])
        self.size_combo.setCurrentText(f"{initial_rows} x {initial_cols}")
        self.size_combo.currentTextChanged.connect(self._on_size_changed)
        ctrl_layout.addWidget(self.size_combo)

        ctrl_layout.addStretch()

        self.info_label = QLabel(f"{self.rows} x {self.cols} cells")
        self.info_label.setObjectName("pathLabel")
        ctrl_layout.addWidget(self.info_label)

        layout.addLayout(ctrl_layout)

        # 2D Grid Frame Container
        self.grid_container = QFrame()
        self.grid_container.setObjectName("gridContainer")
        self.grid_container.setStyleSheet(
            "QFrame#gridContainer { background-color: #0a0a0a; border: 1px solid #333333; border-radius: 4px; }"
        )
        self.grid_layout = QGridLayout(self.grid_container)
        self.grid_layout.setSpacing(2)
        self.grid_layout.setContentsMargins(4, 4, 4, 4)

        layout.addWidget(self.grid_container, 1)
        self.rebuild_grid()

    def _on_size_changed(self, text):
        try:
            r_str, c_str = text.split("x")
            self.rows = int(r_str.strip())
            self.cols = int(c_str.strip())
            self.info_label.setText(f"{self.rows} x {self.cols} cells")
            self.rebuild_grid()
        except ValueError:
            pass

    def rebuild_grid(self):
        while self.grid_layout.count():
            item = self.grid_layout.takeAt(0)
            if item.widget():
                item.widget().deleteLater()

        for r in range(self.rows):
            for c in range(self.cols):
                cell = QFrame()
                cell.setFrameShape(QFrame.StyledPanel)
                cell.setStyleSheet("""
                    QFrame {
                        background-color: #000000;
                        border: 1px solid #1e1e1e;
                        border-radius: 2px;
                    }
                    QFrame:hover {
                        background-color: #111111;
                        border: 1px solid #48a8e6;
                    }
                """)
                cell.setToolTip(f"Cell ({r}, {c})")
                self.grid_layout.addWidget(cell, r, c)


class GridApplication(PortalApplication):
    application_id = "grid"
    display_name = "2D Grid"
    icon = "▦"
    default_size = (580, 460)
    layout_policy = WindowLayoutPolicy(min_width=320, min_height=260)
    behavior_policy = WindowBehaviorPolicy()

    def create_widget(self, portal):
        return GridWidget(portal)


# ---------------------------------------------------------------------------
# Task Viewer Application
# ---------------------------------------------------------------------------

class TaskViewerWidget(QWidget):
    """Real-time task and window inspection tool with interactive task controls."""
    def __init__(self, portal):
        super().__init__()
        self.portal = portal

        layout = QVBoxLayout(self)
        layout.setContentsMargins(8, 8, 8, 8)
        layout.setSpacing(6)

        # Statistics / Metric Cards Header
        stats_frame = QFrame()
        stats_frame.setObjectName("taskStatsFrame")
        stats_layout = QHBoxLayout(stats_frame)
        stats_layout.setContentsMargins(8, 6, 8, 6)
        stats_layout.setSpacing(12)

        self.lbl_windows = QLabel("Windows: 0")
        self.lbl_windows.setObjectName("taskMetricBadge")
        stats_layout.addWidget(self.lbl_windows)

        self.lbl_panes = QLabel("Panes: 0")
        self.lbl_panes.setObjectName("taskMetricBadge")
        stats_layout.addWidget(self.lbl_panes)

        self.lbl_focused = QLabel("Active: None")
        self.lbl_focused.setObjectName("taskMetricBadge")
        stats_layout.addWidget(self.lbl_focused)

        self.lbl_mem = QLabel("Simulated RAM: 32.0 MB")
        self.lbl_mem.setObjectName("taskMetricBadge")
        stats_layout.addWidget(self.lbl_mem)

        stats_layout.addStretch()

        layout.addWidget(stats_frame)

        # Controls & Search Toolbar
        toolbar = QHBoxLayout()
        toolbar.setSpacing(6)

        self.search_input = QLineEdit()
        self.search_input.setObjectName("taskSearchInput")
        self.search_input.setPlaceholderText("🔍 Filter tasks by name/app...")
        self.search_input.textChanged.connect(self.populate_tasks)
        toolbar.addWidget(self.search_input, 1)

        self.btn_refresh = QPushButton("🔄 Refresh")
        self.btn_refresh.clicked.connect(self.populate_tasks)
        toolbar.addWidget(self.btn_refresh)

        self.btn_focus = QPushButton("⌖ Focus")
        self.btn_focus.setToolTip("Focus selected window")
        self.btn_focus.clicked.connect(self.focus_selected)
        toolbar.addWidget(self.btn_focus)

        self.btn_min = QPushButton("— Min")
        self.btn_min.setToolTip("Toggle minimize for selected window")
        self.btn_min.clicked.connect(self.toggle_min_selected)
        toolbar.addWidget(self.btn_min)

        self.btn_max = QPushButton("□ Max")
        self.btn_max.setToolTip("Toggle maximize for selected window")
        self.btn_max.clicked.connect(self.toggle_max_selected)
        toolbar.addWidget(self.btn_max)

        self.btn_undock = QPushButton("⇱ Undock")
        self.btn_undock.setToolTip("Undock selected pane into standalone window")
        self.btn_undock.clicked.connect(self.undock_selected)
        toolbar.addWidget(self.btn_undock)

        self.btn_close = QPushButton("× End Task")
        self.btn_close.setToolTip("Close selected pane or window")
        self.btn_close.clicked.connect(self.close_selected)
        toolbar.addWidget(self.btn_close)

        layout.addLayout(toolbar)

        # Task Tree View
        self.tree = QTreeWidget()
        self.tree.setObjectName("taskTree")
        self.tree.setColumnCount(6)
        self.tree.setHeaderLabels(["Task / Component", "Type", "App ID", "State", "Dimensions", "Panes"])
        self.tree.setRootIsDecorated(True)
        self.tree.setAnimated(True)
        self.tree.setContextMenuPolicy(Qt.CustomContextMenu)
        self.tree.customContextMenuRequested.connect(self.show_context_menu)
        self.tree.itemDoubleClicked.connect(self.item_double_clicked)
        self.tree.header().setSectionResizeMode(0, QHeaderView.Stretch)
        for c in (1, 2, 3, 4, 5):
            self.tree.header().setSectionResizeMode(c, QHeaderView.ResizeToContents)

        layout.addWidget(self.tree, 1)

        # Auto-refresh timer
        self.timer = QTimer(self)
        self.timer.timeout.connect(self.populate_tasks)
        self.timer.start(1000)

        # Initial populate
        self.populate_tasks()

    def get_selected_target(self):
        item = self.tree.currentItem()
        if not item:
            return None, None
        data = item.data(0, Qt.UserRole)
        if not data:
            return None, None
        return data.get("type"), data.get("obj")

    def populate_tasks(self):
        filter_text = self.search_input.text().lower()
        windows = getattr(self.portal.desktop, "windows", [])
        focused = getattr(self.portal.desktop, "_focused", None)

        total_panes = sum(len(w.get_all_panes()) for w in windows)
        self.lbl_windows.setText(f"Windows: {len(windows)}")
        self.lbl_panes.setText(f"Panes: {total_panes}")
        focused_name = (focused.app.display_name if focused and focused.app else "None") if focused else "None"
        self.lbl_focused.setText(f"Active: {focused_name}")
        self.lbl_mem.setText(f"Simulated RAM: {32.0 + len(windows) * 8.4 + total_panes * 4.2:.1f} MB")

        selected_type, selected_obj = self.get_selected_target()
        self.tree.clear()

        for idx, win in enumerate(windows):
            panes = win.get_all_panes()
            is_focused = win is focused
            state_str = win.state.name
            if is_focused and win.state != WindowState.MINIMIZED:
                state_str += " (Focused)"

            if len(panes) == 1:
                p = panes[0]
                title = f"{p.app.icon}  {p.app.display_name}" if p.app else "◫ Split Slot"
                app_id = p.app.application_id if p.app else "placeholder"
                win_type = "Window (Single)"
            else:
                titles = [f"{p.app.icon} {p.app.display_name}" if p.app else "◫" for p in panes]
                title = f"◫ Group [{', '.join(titles)}]"
                app_ids = [p.app.application_id if p.app else "placeholder" for p in panes]
                app_id = "+".join(app_ids)
                win_type = "Window (Grouped)"

            geom_str = f"{win.width()}x{win.height()} @ ({win.x()},{win.y()})"

            if filter_text and filter_text not in title.lower() and filter_text not in app_id.lower() and filter_text not in state_str.lower():
                continue

            win_item = QTreeWidgetItem([
                title,
                win_type,
                app_id,
                state_str,
                geom_str,
                str(len(panes))
            ])
            win_item.setData(0, Qt.UserRole, {"type": "window", "obj": win})
            self.tree.addTopLevelItem(win_item)
            win_item.setExpanded(True)

            if selected_obj == win:
                self.tree.setCurrentItem(win_item)

            if len(panes) > 1:
                for p_idx, pane in enumerate(panes):
                    p_title = f"{pane.app.icon}  {pane.app.display_name}" if pane.app else "◫ Split Slot"
                    p_app = pane.app.application_id if pane.app else "placeholder"
                    p_geom = f"{pane.width()}x{pane.height()}"
                    pane_item = QTreeWidgetItem([
                        f"  ↳ Pane #{p_idx+1}: {p_title}",
                        "Sub-Pane",
                        p_app,
                        "Active",
                        p_geom,
                        "1"
                    ])
                    pane_item.setData(0, Qt.UserRole, {"type": "pane", "obj": pane, "window": win})
                    win_item.addChild(pane_item)

                    if selected_obj == pane:
                        self.tree.setCurrentItem(pane_item)

    def item_double_clicked(self, item, col):
        self.focus_selected()

    def focus_selected(self):
        target_type, target_obj = self.get_selected_target()
        if not target_obj:
            return
        if target_type == "window":
            if target_obj.state == WindowState.MINIMIZED:
                self.portal.desktop.restore(target_obj)
            else:
                self.portal.desktop.focus_window(target_obj)
        elif target_type == "pane":
            win = target_obj.get_portal_window()
            if win:
                if win.state == WindowState.MINIMIZED:
                    self.portal.desktop.restore(win)
                else:
                    self.portal.desktop.focus_window(win)
        self.populate_tasks()

    def toggle_min_selected(self):
        target_type, target_obj = self.get_selected_target()
        if not target_obj:
            return
        win = target_obj if target_type == "window" else target_obj.get_portal_window()
        if win:
            if win.state == WindowState.MINIMIZED:
                self.portal.desktop.restore(win)
            else:
                self.portal.desktop.minimize(win)
        self.populate_tasks()

    def toggle_max_selected(self):
        target_type, target_obj = self.get_selected_target()
        if not target_obj:
            return
        win = target_obj if target_type == "window" else target_obj.get_portal_window()
        if win:
            self.portal.desktop.toggle_maximize(win)
        self.populate_tasks()

    def undock_selected(self):
        target_type, target_obj = self.get_selected_target()
        if target_type == "pane" and target_obj:
            win = target_obj.get_portal_window()
            if win:
                win.undock_pane(target_obj)
        elif target_type == "window" and target_obj:
            panes = target_obj.get_all_panes()
            if len(panes) > 1:
                target_obj.undock_pane(panes[0])
        self.populate_tasks()

    def close_selected(self):
        target_type, target_obj = self.get_selected_target()
        if target_type == "pane" and target_obj:
            win = target_obj.get_portal_window()
            if win:
                win.close_pane(target_obj)
        elif target_type == "window" and target_obj:
            self.portal.desktop.remove_window(target_obj)
        self.populate_tasks()

    def show_context_menu(self, pos):
        item = self.tree.itemAt(pos)
        if not item:
            return
        data = item.data(0, Qt.UserRole)
        if not data:
            return

        menu = QMenu(self)
        menu.addAction("⌖ Focus").triggered.connect(self.focus_selected)
        menu.addAction("— Minimize / Restore").triggered.connect(self.toggle_min_selected)
        menu.addAction("□ Maximize / Restore").triggered.connect(self.toggle_max_selected)
        menu.addSeparator()

        if data.get("type") == "pane":
            menu.addAction("⇱ Undock Pane").triggered.connect(self.undock_selected)
            menu.addAction("× Close Pane").triggered.connect(self.close_selected)
        else:
            menu.addAction("× Close Window").triggered.connect(self.close_selected)

        menu.exec(self.tree.mapToGlobal(pos))


class TaskViewerApplication(PortalApplication):
    application_id = "tasks"
    display_name = "Task Viewer"
    icon = "▤"
    default_size = (680, 460)
    layout_policy = WindowLayoutPolicy(min_width=420, min_height=280)
    behavior_policy = WindowBehaviorPolicy()

    def create_widget(self, portal):
        return TaskViewerWidget(portal)


# ---------------------------------------------------------------------------
# Console Output Application
# ---------------------------------------------------------------------------

class ConsoleWidget(QWidget):
    """Interactive system event console and simulator command shell."""
    def __init__(self, portal):
        super().__init__()
        self.portal = portal
        self.history_index = 0
        self.command_history = []

        layout = QVBoxLayout(self)
        layout.setContentsMargins(8, 8, 8, 8)
        layout.setSpacing(6)

        # Toolbar
        toolbar = QHBoxLayout()
        toolbar.setSpacing(6)

        self.filter_combo = QComboBox()
        self.filter_combo.addItems(["All Logs", "System [SYS]", "Events [EVENT]", "Actions [ACTION]", "Warnings [WARN]"])
        self.filter_combo.currentTextChanged.connect(self.render_logs)
        toolbar.addWidget(self.filter_combo)

        self.search_input = QLineEdit()
        self.search_input.setPlaceholderText("🔍 Filter logs...")
        self.search_input.textChanged.connect(self.render_logs)
        toolbar.addWidget(self.search_input, 1)

        self.btn_clear = QPushButton("⎚ Clear")
        self.btn_clear.clicked.connect(self.clear_logs)
        toolbar.addWidget(self.btn_clear)

        self.btn_copy = QPushButton("📋 Copy")
        self.btn_copy.clicked.connect(self.copy_logs)
        toolbar.addWidget(self.btn_copy)

        self.autoscroll_chk = QCheckBox("Auto-Scroll")
        self.autoscroll_chk.setChecked(True)
        toolbar.addWidget(self.autoscroll_chk)

        layout.addLayout(toolbar)

        # Log Display Window
        self.text_area = QTextEdit()
        self.text_area.setReadOnly(True)
        self.text_area.setObjectName("consoleOutputArea")
        self.text_area.setFont(QFont("Consolas", 9))
        layout.addWidget(self.text_area, 1)

        # Command Input Bar
        cmd_layout = QHBoxLayout()
        cmd_layout.setSpacing(6)

        self.prompt_label = QLabel("portal>")
        self.prompt_label.setObjectName("consolePromptLabel")
        cmd_layout.addWidget(self.prompt_label)

        self.cmd_input = QLineEdit()
        self.cmd_input.setObjectName("consoleCmdInput")
        self.cmd_input.setPlaceholderText("Type a command (e.g. 'help', 'launch files', 'list', 'theme Classic')...")
        self.cmd_input.returnPressed.connect(self.execute_command)
        cmd_layout.addWidget(self.cmd_input, 1)

        self.btn_run = QPushButton("↵ Run")
        self.btn_run.setObjectName("consoleRunButton")
        self.btn_run.clicked.connect(self.execute_command)
        cmd_layout.addWidget(self.btn_run)

        layout.addLayout(cmd_layout)

        # Connect live log event
        self.portal.eventLogged.connect(self.on_event_logged)

        # Render initial logs
        self.render_logs()
        self.append_system_message("Portal Console v2.0 ready. Type 'help' for available commands.")

    def on_event_logged(self, entry):
        self.render_logs()

    def clear_logs(self):
        self.portal.log_history.clear()
        self.text_area.clear()
        self.append_system_message("Console log cleared.")

    def copy_logs(self):
        QApplication.clipboard().setText(self.text_area.toPlainText())
        self.portal.status("Console logs copied to clipboard.")

    def append_system_message(self, text):
        self.text_area.append(f"<span style='color: #4da6ff; font-weight: bold;'>[SYS]</span> <span style='color: #888;'>{datetime.datetime.now().strftime('%H:%M:%S')}</span> &nbsp; <span style='color: #eeeeee;'>{text}</span>")
        if self.autoscroll_chk.isChecked():
            self.text_area.verticalScrollBar().setValue(self.text_area.verticalScrollBar().maximum())

    def render_logs(self):
        filter_type = self.filter_combo.currentText()
        filter_query = self.search_input.text().lower()

        html_lines = []
        for item in self.portal.log_history:
            level = item.get("level", "INFO")
            ts = item.get("timestamp", "")
            cat = item.get("category", "")
            msg = item.get("message", "")

            if "System" in filter_type and level != "SYS":
                continue
            if "Events" in filter_type and level != "EVENT":
                continue
            if "Actions" in filter_type and level != "ACTION":
                continue
            if "Warnings" in filter_type and level != "WARN":
                continue

            full_str = f"{level} {cat} {msg}".lower()
            if filter_query and filter_query not in full_str:
                continue

            if level == "SYS":
                badge = "<span style='color: #4da6ff; font-weight: bold;'>[SYS]</span>"
            elif level == "EVENT":
                badge = "<span style='color: #4cd964; font-weight: bold;'>[EVENT]</span>"
            elif level == "ACTION":
                badge = "<span style='color: #ff9f0a; font-weight: bold;'>[ACTION]</span>"
            elif level == "WARN":
                badge = "<span style='color: #ff453a; font-weight: bold;'>[WARN]</span>"
            else:
                badge = "<span style='color: #8e8e93;'>[INFO]</span>"

            cat_badge = f"<span style='color: #a0a0a0;'>({cat})</span>" if cat else ""
            line = f"{badge} <span style='color: #666;'>{ts}</span> {cat_badge} <span style='color: #dddddd;'>{msg}</span>"
            html_lines.append(line)

        self.text_area.setHtml("<br>".join(html_lines))
        if self.autoscroll_chk.isChecked():
            self.text_area.verticalScrollBar().setValue(self.text_area.verticalScrollBar().maximum())

    def execute_command(self):
        cmd = self.cmd_input.text().strip()
        if not cmd:
            return
        self.cmd_input.clear()
        self.command_history.append(cmd)
        self.history_index = len(self.command_history)

        self.portal.log("ACTION", "CMD", f"> {cmd}")

        parts = cmd.split()
        verb = parts[0].lower()
        args = parts[1:]

        try:
            if verb in ("help", "?"):
                help_text = (
                    "<b>Available Commands:</b><br>"
                    "&nbsp;&nbsp;<b>launch &lt;app_id&gt;</b> - Launch app (home, files, grid, tasks, console, settings, about)<br>"
                    "&nbsp;&nbsp;<b>list</b> - List all open windows and panes<br>"
                    "&nbsp;&nbsp;<b>focus &lt;id&gt;</b> - Focus window by index (1-based)<br>"
                    "&nbsp;&nbsp;<b>close &lt;id&gt;</b> - Close window by index (1-based)<br>"
                    "&nbsp;&nbsp;<b>min &lt;id&gt;</b> - Minimize window by index<br>"
                    "&nbsp;&nbsp;<b>max &lt;id&gt;</b> - Toggle maximize window by index<br>"
                    "&nbsp;&nbsp;<b>split &lt;id&gt; [h|v]</b> - Split window horizontally or vertically<br>"
                    "&nbsp;&nbsp;<b>undock &lt;win_id&gt; [pane_idx]</b> - Undock a pane from window<br>"
                    "&nbsp;&nbsp;<b>theme [name]</b> - List themes or switch to a theme<br>"
                    "&nbsp;&nbsp;<b>setting &lt;key&gt; &lt;val&gt;</b> - Update simulator setting<br>"
                    "&nbsp;&nbsp;<b>stats</b> - Display system statistics<br>"
                    "&nbsp;&nbsp;<b>clear</b> - Clear console output<br>"
                    "&nbsp;&nbsp;<b>echo &lt;msg&gt;</b> - Print a custom message"
                )
                self.append_system_message(help_text)

            elif verb == "launch":
                if not args:
                    self.append_system_message("Usage: launch &lt;app_id&gt; (home, files, grid, tasks, console, settings, about)")
                else:
                    aid = args[0].lower()
                    if aid in self.portal.apps:
                        self.portal.launch(aid)
                        self.append_system_message(f"Launched {self.portal.apps[aid].display_name} successfully.")
                    else:
                        self.append_system_message(f"Unknown app '{aid}'. Available apps: {', '.join(self.portal.apps.keys())}")

            elif verb == "list":
                windows = self.portal.desktop.windows
                if not windows:
                    self.append_system_message("No open windows.")
                else:
                    lines = ["<b>Active Windows:</b>"]
                    for idx, w in enumerate(windows):
                        panes = w.get_all_panes()
                        focused_str = " [FOCUSED]" if w is self.portal.desktop._focused else ""
                        p_names = [p.app.display_name if p.app else "Split Slot" for p in panes]
                        lines.append(f"&nbsp;&nbsp;#{idx+1}: {', '.join(p_names)} ({w.state.name}){focused_str} - {w.width()}x{w.height()}")
                    self.append_system_message("<br>".join(lines))

            elif verb == "focus":
                if not args or not args[0].isdigit():
                    self.append_system_message("Usage: focus &lt;window_number&gt;")
                else:
                    idx = int(args[0]) - 1
                    if 0 <= idx < len(self.portal.desktop.windows):
                        w = self.portal.desktop.windows[idx]
                        if w.state == WindowState.MINIMIZED:
                            self.portal.desktop.restore(w)
                        else:
                            self.portal.desktop.focus_window(w)
                        self.append_system_message(f"Focused Window #{idx+1}.")
                    else:
                        self.append_system_message(f"Invalid window index {args[0]}.")

            elif verb == "close":
                if not args or not args[0].isdigit():
                    self.append_system_message("Usage: close &lt;window_number&gt;")
                else:
                    idx = int(args[0]) - 1
                    if 0 <= idx < len(self.portal.desktop.windows):
                        w = self.portal.desktop.windows[idx]
                        self.portal.desktop.remove_window(w)
                        self.append_system_message(f"Closed Window #{idx+1}.")
                    else:
                        self.append_system_message(f"Invalid window index {args[0]}.")

            elif verb == "min":
                if not args or not args[0].isdigit():
                    self.append_system_message("Usage: min &lt;window_number&gt;")
                else:
                    idx = int(args[0]) - 1
                    if 0 <= idx < len(self.portal.desktop.windows):
                        w = self.portal.desktop.windows[idx]
                        self.portal.desktop.minimize(w)
                        self.append_system_message(f"Minimized Window #{idx+1}.")
                    else:
                        self.append_system_message(f"Invalid window index {args[0]}.")

            elif verb == "max":
                if not args or not args[0].isdigit():
                    self.append_system_message("Usage: max &lt;window_number&gt;")
                else:
                    idx = int(args[0]) - 1
                    if 0 <= idx < len(self.portal.desktop.windows):
                        w = self.portal.desktop.windows[idx]
                        self.portal.desktop.toggle_maximize(w)
                        self.append_system_message(f"Toggled maximize for Window #{idx+1} ({w.state.name}).")
                    else:
                        self.append_system_message(f"Invalid window index {args[0]}.")

            elif verb == "split":
                if not args or not args[0].isdigit():
                    self.append_system_message("Usage: split &lt;window_number&gt; [h|v]")
                else:
                    idx = int(args[0]) - 1
                    if 0 <= idx < len(self.portal.desktop.windows):
                        w = self.portal.desktop.windows[idx]
                        orient_str = args[1].lower() if len(args) > 1 else "h"
                        orient = Qt.Vertical if orient_str in ("v", "vertical", "down") else Qt.Horizontal
                        w.split(orient, "after")
                        self.append_system_message(f"Split Window #{idx+1} ({'Vertical' if orient == Qt.Vertical else 'Horizontal'}).")
                    else:
                        self.append_system_message(f"Invalid window index {args[0]}.")

            elif verb == "undock":
                if not args or not args[0].isdigit():
                    self.append_system_message("Usage: undock &lt;window_number&gt; [pane_number]")
                else:
                    idx = int(args[0]) - 1
                    if 0 <= idx < len(self.portal.desktop.windows):
                        w = self.portal.desktop.windows[idx]
                        panes = w.get_all_panes()
                        p_idx = int(args[1]) - 1 if len(args) > 1 and args[1].isdigit() else 0
                        if 0 <= p_idx < len(panes):
                            w.undock_pane(panes[p_idx])
                            self.append_system_message(f"Undocked pane #{p_idx+1} from Window #{idx+1}.")
                        else:
                            self.append_system_message(f"Invalid pane index {p_idx+1}.")
                    else:
                        self.append_system_message(f"Invalid window index {args[0]}.")

            elif verb == "theme":
                if not args:
                    themes_str = ", ".join(THEMES.keys())
                    self.append_system_message(f"Current theme: <b>{self.portal.theme.name}</b><br>Available themes: {themes_str}")
                else:
                    t_name = " ".join(args)
                    matched = None
                    for k in THEMES:
                        if k.lower() == t_name.lower():
                            matched = k
                            break
                    if matched:
                        self.portal.set_theme(matched)
                        self.append_system_message(f"Theme set to '{matched}'.")
                    else:
                        self.append_system_message(f"Unknown theme '{t_name}'. Options: {', '.join(THEMES.keys())}")

            elif verb == "setting":
                if len(args) < 2:
                    self.append_system_message(f"Current settings: {self.portal.settings}")
                else:
                    k = args[0]
                    v_raw = args[1]
                    if v_raw.lower() in ("true", "1", "yes"):
                        v = True
                    elif v_raw.lower() in ("false", "0", "no"):
                        v = False
                    elif v_raw.isdigit():
                        v = int(v_raw)
                    else:
                        v = v_raw
                    self.portal.update_setting(k, v)
                    self.append_system_message(f"Updated setting '{k}' to {v}.")

            elif verb == "clear":
                self.clear_logs()

            elif verb == "echo":
                self.append_system_message(" ".join(args))

            elif verb == "stats":
                windows = self.portal.desktop.windows
                total_panes = sum(len(w.get_all_panes()) for w in windows)
                stats_info = (
                    f"<b>BSPT Portal Status:</b><br>"
                    f"&nbsp;&nbsp;Desktop Resolution: {self.portal.desktop.width()}x{self.portal.desktop.height()}<br>"
                    f"&nbsp;&nbsp;Theme: {self.portal.theme.name}<br>"
                    f"&nbsp;&nbsp;Active Windows: {len(windows)}<br>"
                    f"&nbsp;&nbsp;Total Panes: {total_panes}<br>"
                    f"&nbsp;&nbsp;Registered Apps: {len(self.portal.apps)}<br>"
                    f"&nbsp;&nbsp;Operator Mode: {getattr(self.portal.business_state, 'operator_mode', 'N/A').name if hasattr(self.portal, 'business_state') else 'N/A'}"
                )
                self.append_system_message(stats_info)

            elif verb == "mode":
                if not args:
                    curr = self.portal.business_state.operator_mode.name if hasattr(self.portal, "business_state") else "N/A"
                    self.append_system_message(f"Current Operator Mode: <b>{curr}</b>. Usage: mode &lt;green|yellow|red&gt;")
                else:
                    m_str = args[0].lower()
                    target_mode = None
                    if m_str == "green": target_mode = OperatorMode.GREEN
                    elif m_str == "yellow": target_mode = OperatorMode.YELLOW
                    elif m_str == "red": target_mode = OperatorMode.RED
                    if target_mode:
                        self.portal.business_state.set_operator_mode(target_mode)
                        self.append_system_message(f"Operator mode switched to <b>{target_mode.name}</b>.")
                    else:
                        self.append_system_message("Invalid mode. Options: green, yellow, red.")

            elif verb == "plan":
                msg = self.portal.business_state.generate_daily_plan()
                self.append_system_message(f"Daily Planner: {msg}")

            elif verb == "report":
                rep = self.portal.business_state.generate_return_report()
                self.append_system_message(f"Return Report <b>{rep.id}</b> generated at {rep.timestamp}. Revenue: +${rep.revenue_generated:.2f} | Cost: -${rep.costs_incurred:.2f}")

            elif verb == "blockers":
                blks = self.portal.business_state.blockers.values()
                if not blks:
                    self.append_system_message("No physical blockers.")
                else:
                    lines = ["<b>Active Physical Blockers (Delegation Queue):</b>"]
                    for b in blks:
                        lines.append(f"&nbsp;&nbsp;<b>{b.id}</b>: {b.task_title} [{b.physical_cost.name}] - Status: {b.delegation_status} (Est: {b.est_time})")
                    self.append_system_message("<br>".join(lines))

            elif verb == "approvals":
                apprs = self.portal.business_state.approvals.values()
                if not apprs:
                    self.append_system_message("No pending approval requests.")
                else:
                    lines = ["<b>Pending Authorization Requests:</b>"]
                    for a in apprs:
                        lines.append(f"&nbsp;&nbsp;<b>{a.id}</b> ({a.risk_level}): {a.description} [{a.status}] - Requested by: {a.requested_by}")
                    self.append_system_message("<br>".join(lines))

            elif verb == "approve":
                if not args:
                    self.append_system_message("Usage: approve &lt;approval_id&gt; (e.g. APP-01)")
                else:
                    aid = args[0].upper()
                    msg = self.portal.business_state.resolve_approval(aid, True)
                    self.append_system_message(msg)

            elif verb == "reject":
                if not args:
                    self.append_system_message("Usage: reject &lt;approval_id&gt; (e.g. APP-01)")
                else:
                    aid = args[0].upper()
                    msg = self.portal.business_state.resolve_approval(aid, False)
                    self.append_system_message(msg)

            else:
                self.append_system_message(f"Unknown command '{verb}'. Type 'help' for available commands.")

        except Exception as e:
            self.append_system_message(f"<span style='color: #ff453a;'>Error executing '{cmd}': {str(e)}</span>")


class ConsoleApplication(PortalApplication):
    application_id = "console"
    display_name = "Console Output"
    icon = "⌨"
    default_size = (640, 420)
    layout_policy = WindowLayoutPolicy(min_width=380, min_height=260)
    behavior_policy = WindowBehaviorPolicy()

    def create_widget(self, portal):
        return ConsoleWidget(portal)


# ---------------------------------------------------------------------------
# Business Autonomy System — Data Models & State Engine
# ---------------------------------------------------------------------------

class OperatorMode(Enum):
    GREEN = "GREEN (Active / Full Operations)"
    YELLOW = "YELLOW (Limited Physical Capacity — Software Only)"
    RED = "RED (Operator Unavailable — Safe Continuity Mode)"


class PhysicalCost(Enum):
    GREEN = "🟢 GREEN (Software-Only)"
    BLUE = "🔵 BLUE (Minimal Physical < 2m)"
    ORANGE = "🟠 ORANGE (Substantial Physical Assist)"
    RED = "🔴 RED (Heavy / Professional Physical)"


class ModelLevel(Enum):
    L0_CODE = "Level 0 — Deterministic Code"
    L1_LOCAL = "Level 1 — Local Model (Ollama)"
    L2_CHEAP_CLOUD = "Level 2 — Cheap Cloud Model"
    L3_HIGH_END = "Level 3 — High-End Frontier Model"


class TaskCategory(Enum):
    MUST_DO = "MUST DO (Critical)"
    REVENUE = "REVENUE (Direct Value)"
    BIZ_DEV = "BIZ DEV (Future Growth)"
    MAINTENANCE = "MAINTENANCE (Infrastructure)"
    RESEARCH = "RESEARCH (Investigation)"
    OPTIONAL = "OPTIONAL (Backlog)"


class TaskStatus(Enum):
    PENDING = "Pending"
    IN_PROGRESS = "In Progress"
    BLOCKED_PHYSICAL = "Blocked (Physical)"
    AWAITING_APPROVAL = "Awaiting Approval"
    QA_REVIEW = "QA Review"
    COMPLETED = "Completed"


@dataclass
class ProjectRecord:
    id: str
    name: str
    objective: str
    status: str
    priority: str
    revenue_potential: str
    physical_req: str
    owner: str
    next_action: str


@dataclass
class BusinessTaskRecord:
    id: str
    project_id: str
    title: str
    category: TaskCategory
    physical_cost: PhysicalCost
    model_level: ModelLevel
    worker_role: str
    status: TaskStatus
    est_minutes: int
    cost_est: float
    output: str = ""
    qa_status: str = "Unverified"
    notes: str = ""


@dataclass
class PhysicalBlockerRecord:
    id: str
    task_id: str
    task_title: str
    physical_cost: PhysicalCost
    description: str
    tools_required: str
    safety_notes: str
    business_value: str
    est_time: str
    deadline: str
    delegation_status: str = "Ready for Helper"


@dataclass
class ApprovalRecord:
    id: str
    task_id: str
    action_type: str
    description: str
    risk_level: str  # "LOW", "MEDIUM", "HIGH"
    status: str      # "PENDING", "APPROVED", "REJECTED"
    requested_by: str
    details: str
    timestamp: str


@dataclass
class RevenueOpportunityRecord:
    id: str
    title: str
    category: str
    customer_target: str
    estimated_value: float
    effort_score: float      # 1 to 10
    probability: float       # 0.0 to 1.0
    reusability_score: float # 1.0 to 5.0
    automation_potential: str
    physical_requirement: PhysicalCost

    @property
    def roi_score(self) -> float:
        if self.effort_score <= 0:
            return 0.0
        return round((self.estimated_value * self.probability * self.reusability_score) / self.effort_score, 2)


@dataclass
class WorkerAgentRecord:
    id: str
    role_name: str
    default_tier: ModelLevel
    status: str  # "Idle", "Active", "Waiting Approval", "Standby"
    current_task: str
    tasks_completed: int
    tokens_used: int
    cost_incurred: float


@dataclass
class DecisionRecord:
    id: str
    date_str: str
    decision: str
    reason: str
    alternatives: str
    consequence: str


@dataclass
class LessonRecord:
    id: str
    date_str: str
    failure: str
    root_cause: str
    correction: str
    future_rule: str


@dataclass
class ReturnReportRecord:
    id: str
    timestamp: str
    mode_during_absence: str
    duration_hours: float
    tasks_completed: list
    tasks_failed_or_blocked: list
    revenue_generated: float
    costs_incurred: float
    physical_blockers_found: list
    decisions_made: list
    human_action_items: list
    next_recommended_plan: list


class BusinessStateEngine:
    """Central persistent business state, model routing, and operator capacity engine."""
    def __init__(self, portal):
        self.portal = portal
        self.operator_mode = OperatorMode.GREEN
        # All metrics start at zero — populated by real connected data sources only
        self.total_tokens = 0
        self.total_cost = 0.0
        self.value_generated = 0.0

        self.projects: dict[str, ProjectRecord] = {}
        self.tasks: dict[str, BusinessTaskRecord] = {}
        self.blockers: dict[str, PhysicalBlockerRecord] = {}
        self.approvals: dict[str, ApprovalRecord] = {}
        self.revenue_opportunities: dict[str, RevenueOpportunityRecord] = {}
        self.workers: dict[str, WorkerAgentRecord] = {}
        self.decisions: list[DecisionRecord] = []
        self.lessons: list[LessonRecord] = []
        self.return_reports: list[ReturnReportRecord] = []

    def set_operator_mode(self, mode: OperatorMode):
        old_mode = self.operator_mode
        self.operator_mode = mode
        self.portal.log("EVENT", "OPERATOR_MODE", f"Operator mode switched from {old_mode.name} to {mode.name}")

        if mode == OperatorMode.YELLOW:
            deferred_count = 0
            for t in self.tasks.values():
                if t.status in (TaskStatus.PENDING, TaskStatus.IN_PROGRESS) and t.physical_cost != PhysicalCost.GREEN:
                    t.status = TaskStatus.BLOCKED_PHYSICAL
                    deferred_count += 1
            self.portal.log("ACTION", "AUTONOMY", f"YELLOW mode active: {deferred_count} physical tasks routed to Delegation Queue.")

        elif mode == OperatorMode.RED:
            self.portal.log("ACTION", "AUTONOMY", "RED continuity mode engaged. Halting all destructive or high-risk tasks. Maintaining digital progress.")

        elif mode == OperatorMode.GREEN and old_mode == OperatorMode.RED:
            self.generate_return_report()

    def advance_task(self, task_id: str) -> str:
        task = self.tasks.get(task_id)
        if not task:
            return "Task not found."

        if task.status == TaskStatus.COMPLETED:
            return f"Task '{task.title}' is already completed."

        if task.physical_cost != PhysicalCost.GREEN and self.operator_mode in (OperatorMode.YELLOW, OperatorMode.RED):
            task.status = TaskStatus.BLOCKED_PHYSICAL
            self.portal.log("WARN", "PHYSICAL_BLOCK", f"Task '{task.title}' requires physical intervention ({task.physical_cost.name}). Blocked in {self.operator_mode.name} mode.")
            return f"Task blocked by physical requirement ({task.physical_cost.name}). Added to Delegation Queue."

        if task.status == TaskStatus.PENDING:
            task.status = TaskStatus.IN_PROGRESS
            msg = f"Started task '{task.title}' under {task.worker_role} ({task.model_level.name})."
        elif task.status == TaskStatus.IN_PROGRESS:
            if task.category == TaskCategory.REVENUE and task.cost_est > 0.03:
                task.status = TaskStatus.AWAITING_APPROVAL
                msg = f"Task '{task.title}' completed draft. Awaiting operator approval."
            else:
                task.status = TaskStatus.QA_REVIEW
                msg = f"Task '{task.title}' sent to QA verification."
        elif task.status == TaskStatus.QA_REVIEW:
            task.status = TaskStatus.COMPLETED
            task.qa_status = "Verified"
            task.output = f"Successfully executed and verified at {datetime.datetime.now().strftime('%H:%M:%S')}."
            added_tokens = 450 if task.model_level == ModelLevel.L1_LOCAL else (1200 if task.model_level == ModelLevel.L2_CHEAP_CLOUD else 0)
            added_cost = task.cost_est
            self.total_tokens += added_tokens
            self.total_cost += added_cost
            if task.category == TaskCategory.REVENUE:
                self.value_generated += 49.0
            msg = f"Task '{task.title}' verified and completed."
        elif task.status == TaskStatus.AWAITING_APPROVAL:
            msg = f"Task '{task.title}' is awaiting operator authorization."
        elif task.status == TaskStatus.BLOCKED_PHYSICAL:
            msg = f"Task '{task.title}' is physically blocked. Resolve on Delegation Queue."
        else:
            msg = f"Task status updated."

        self.portal.log("ACTION", "TASK", msg)
        return msg

    def resolve_approval(self, approval_id: str, approved: bool) -> str:
        appr = self.approvals.get(approval_id)
        if not appr:
            return "Approval not found."

        appr.status = "APPROVED" if approved else "REJECTED"
        task = self.tasks.get(appr.task_id)
        if task:
            if approved:
                task.status = TaskStatus.QA_REVIEW
                task.output = "Approved by operator. Proceeding to final delivery."
            else:
                task.status = TaskStatus.PENDING
                task.output = "Rejected by operator. Rework required."

        status_str = "Approved" if approved else "Rejected"
        self.portal.log("ACTION", "APPROVAL", f"{status_str} approval {approval_id}: {appr.description}")
        return f"Approval {approval_id} marked as {status_str}."

    def resolve_physical_blocker(self, blocker_id: str) -> str:
        blk = self.blockers.get(blocker_id)
        if not blk:
            return "Blocker not found."

        blk.delegation_status = "Completed by Helper"
        task = self.tasks.get(blk.task_id)
        if task:
            task.status = TaskStatus.IN_PROGRESS
            task.output = "Physical action completed by assistant. Resuming software steps."

        self.portal.log("ACTION", "PHYSICAL", f"Physical blocker {blocker_id} resolved by helper. Task {blk.task_id} resumed.")
        return f"Blocker {blocker_id} marked as resolved."

    def generate_daily_plan(self) -> str:
        completed = sum(1 for t in self.tasks.values() if t.status == TaskStatus.COMPLETED)
        pending = sum(1 for t in self.tasks.values() if t.status in (TaskStatus.PENDING, TaskStatus.IN_PROGRESS))
        blocked = sum(1 for t in self.tasks.values() if t.status == TaskStatus.BLOCKED_PHYSICAL)

        self.portal.log("ACTION", "PLANNER", f"Daily Work Plan generated: {pending} active tasks, {completed} completed, {blocked} physical blockers. Mode: {self.operator_mode.name}.")
        return "Daily plan refreshed successfully."

    def generate_return_report(self) -> ReturnReportRecord:
        rep_id = f"REP-{len(self.return_reports)+1:02d}"
        completed_tasks = [t.title for t in self.tasks.values() if t.status == TaskStatus.COMPLETED]
        blocked_tasks = [f"{b.id}: {b.task_title} ({b.physical_cost.name})" for b in self.blockers.values() if b.delegation_status != "Completed by Helper"]
        human_items = [f"Assistant help: {b.task_title} - Tools: {b.tools_required}" for b in self.blockers.values() if b.delegation_status != "Completed by Helper"]

        report = ReturnReportRecord(
            rep_id,
            datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
            self.operator_mode.name,
            3.2,
            completed_tasks[-4:] if completed_tasks else ["No tasks completed."],
            blocked_tasks,
            self.value_generated,
            round(self.total_cost, 2),
            [f"{len(self.blockers)} physical blockers on Delegation Queue"],
            ["Executed autonomous maintenance loop", "Preserved all data integrity"],
            human_items,
            ["Review pending approvals in Approval Center", "Verify completed research notes", "Check helper delegation status"]
        )
        self.return_reports.insert(0, report)
        self.portal.log("EVENT", "REPORT", f"Generated Return Report {rep_id} ({self.operator_mode.name}).")
        return report


# ---------------------------------------------------------------------------
# Business Supervisor Application
# ---------------------------------------------------------------------------

class BusinessSupervisorWidget(QWidget):
    """Central command center for the operator, daily planner, physical blockers, and approvals."""
    def __init__(self, portal):
        super().__init__()
        self.portal = portal
        self.state: BusinessStateEngine = getattr(portal, "business_state", None)
        if not self.state:
            self.state = BusinessStateEngine(portal)
            portal.business_state = self.state

        layout = QVBoxLayout(self)
        layout.setContentsMargins(8, 8, 8, 8)
        layout.setSpacing(6)

        # 1. Operator Mode & Capacity Status Header
        mode_frame = QFrame()
        mode_frame.setObjectName("operatorModeFrame")
        mode_layout = QVBoxLayout(mode_frame)
        mode_layout.setContentsMargins(10, 8, 10, 8)
        mode_layout.setSpacing(6)

        top_row = QHBoxLayout()
        lbl_m = QLabel("<b>Operator Capacity Mode:</b>")
        lbl_m.setStyleSheet("font-size: 10pt;")
        top_row.addWidget(lbl_m)

        self.btn_green = QPushButton("🟢 GREEN (Active)")
        self.btn_green.setObjectName("modeBtnGreen")
        self.btn_green.setCursor(Qt.PointingHandCursor)
        self.btn_green.clicked.connect(lambda: self.set_mode(OperatorMode.GREEN))
        top_row.addWidget(self.btn_green)

        self.btn_yellow = QPushButton("🟡 YELLOW (Limited Physical)")
        self.btn_yellow.setObjectName("modeBtnYellow")
        self.btn_yellow.setCursor(Qt.PointingHandCursor)
        self.btn_yellow.clicked.connect(lambda: self.set_mode(OperatorMode.YELLOW))
        top_row.addWidget(self.btn_yellow)

        self.btn_red = QPushButton("🔴 RED (Continuity / Away)")
        self.btn_red.setObjectName("modeBtnRed")
        self.btn_red.setCursor(Qt.PointingHandCursor)
        self.btn_red.clicked.connect(lambda: self.set_mode(OperatorMode.RED))
        top_row.addWidget(self.btn_red)

        top_row.addStretch()

        self.btn_step = QPushButton("⚡ Step Sim")
        self.btn_step.setToolTip("Advance next active or pending task")
        self.btn_step.clicked.connect(self.step_next_task)
        top_row.addWidget(self.btn_step)

        self.btn_plan = QPushButton("📋 New Daily Plan")
        self.btn_plan.clicked.connect(self.generate_plan)
        top_row.addWidget(self.btn_plan)

        self.btn_report = QPushButton("📜 Return Report")
        self.btn_report.clicked.connect(self.generate_report)
        top_row.addWidget(self.btn_report)

        mode_layout.addLayout(top_row)

        self.mode_desc = QLabel()
        self.mode_desc.setWordWrap(True)
        mode_layout.addWidget(self.mode_desc)

        layout.addWidget(mode_frame)

        # 2. Tabbed Operator Views
        self.tabs = QTabWidget()
        self.tabs.setObjectName("supervisorTabs")

        # Tab 1: Daily Plan & Tasks
        self.tab_plan = QWidget()
        plan_layout = QVBoxLayout(self.tab_plan)
        plan_layout.setContentsMargins(6, 6, 6, 6)
        plan_layout.setSpacing(6)

        self.task_tree = QTreeWidget()
        self.task_tree.setColumnCount(7)
        self.task_tree.setHeaderLabels(["Task / Title", "Project", "Physical Cost", "Model Tier", "Worker", "Status", "Est Cost"])
        self.task_tree.header().setSectionResizeMode(0, QHeaderView.Stretch)
        for c in (1, 2, 3, 4, 5, 6):
            self.task_tree.header().setSectionResizeMode(c, QHeaderView.ResizeToContents)
        self.task_tree.itemSelectionChanged.connect(self.update_task_status_label)
        self.task_tree.itemDoubleClicked.connect(lambda item, col: self.advance_selected_task())
        plan_layout.addWidget(self.task_tree, 1)

        task_btn_row = QHBoxLayout()
        self.btn_advance = QPushButton("⚡ Advance Selected Task")
        self.btn_advance.clicked.connect(self.advance_selected_task)
        task_btn_row.addWidget(self.btn_advance)

        self.btn_view_task = QPushButton("🔍 View Details / Output")
        self.btn_view_task.clicked.connect(self.view_selected_task)
        task_btn_row.addWidget(self.btn_view_task)

        task_btn_row.addStretch()
        self.task_status_lbl = QLabel()
        self.task_status_lbl.setObjectName("pathLabel")
        task_btn_row.addWidget(self.task_status_lbl)

        plan_layout.addLayout(task_btn_row)
        self.tabs.addTab(self.tab_plan, "📋 Daily Plan & Work Queue")

        # Tab 2: Physical Blockers & Delegation Queue
        self.tab_delegation = QWidget()
        del_layout = QHBoxLayout(self.tab_delegation)
        del_layout.setContentsMargins(6, 6, 6, 6)
        del_layout.setSpacing(8)

        # Left: Blockers List
        left_del = QVBoxLayout()
        left_del.addWidget(QLabel("<b>🚧 Active Physical Blockers:</b>"))
        self.blocker_list = QTreeWidget()
        self.blocker_list.setColumnCount(4)
        self.blocker_list.setHeaderLabels(["ID", "Task Title", "Physical Demand", "Est Time"])
        self.blocker_list.header().setSectionResizeMode(1, QHeaderView.Stretch)
        self.blocker_list.itemSelectionChanged.connect(self.update_delegation_sheet)
        left_del.addWidget(self.blocker_list, 1)
        del_layout.addLayout(left_del, 1)

        # Right: Delegation Brief Sheet Card
        self.del_card = QFrame()
        self.del_card.setObjectName("delegationCard")
        del_card_layout = QVBoxLayout(self.del_card)
        del_card_layout.setContentsMargins(14, 12, 14, 12)
        del_card_layout.setSpacing(8)

        card_title = QLabel("📋 ASSISTANT DELEGATION SHEET")
        card_title.setObjectName("sectionTitle")
        del_card_layout.addWidget(card_title)

        self.del_task_title = QLabel("Select a physical blocker to view delegation instructions.")
        self.del_task_title.setWordWrap(True)
        self.del_task_title.setStyleSheet("font-weight: bold; font-size: 11pt;")
        del_card_layout.addWidget(self.del_task_title)

        self.del_demand_badge = QLabel("Demand: None")
        del_card_layout.addWidget(self.del_demand_badge)

        self.del_why = QLabel()
        self.del_why.setWordWrap(True)
        del_card_layout.addWidget(self.del_why)

        self.del_tools = QLabel()
        self.del_tools.setWordWrap(True)
        del_card_layout.addWidget(self.del_tools)

        self.del_safety = QLabel()
        self.del_safety.setWordWrap(True)
        del_card_layout.addWidget(self.del_safety)

        self.del_value = QLabel()
        self.del_value.setWordWrap(True)
        del_card_layout.addWidget(self.del_value)

        del_card_layout.addStretch()

        self.btn_resolve_blk = QPushButton("✓ Mark Physical Task Completed by Helper")
        self.btn_resolve_blk.setCursor(Qt.PointingHandCursor)
        self.btn_resolve_blk.setStyleSheet("background: #2d6a4f; color: #ffffff; font-weight: bold; padding: 8px 16px;")
        self.btn_resolve_blk.clicked.connect(self.resolve_selected_blocker)
        del_card_layout.addWidget(self.btn_resolve_blk)

        del_layout.addWidget(self.del_card, 1)
        self.tabs.addTab(self.tab_delegation, "🚧 Physical Blockers & Delegation")

        # Tab 3: Approvals & Governance
        self.tab_approvals = QWidget()
        appr_layout = QVBoxLayout(self.tab_approvals)
        appr_layout.setContentsMargins(6, 6, 6, 6)
        appr_layout.setSpacing(6)

        appr_layout.addWidget(QLabel("<b>🛡️ Actions Awaiting Human Operator Authorization:</b>"))
        self.appr_table = QTableWidget()
        self.appr_table.setColumnCount(6)
        self.appr_table.setHorizontalHeaderLabels(["ID", "Task ID", "Action Description", "Risk Level", "Requested By", "Status"])
        self.appr_table.horizontalHeader().setSectionResizeMode(2, QHeaderView.Stretch)
        self.appr_table.itemSelectionChanged.connect(self.update_approval_detail)
        appr_layout.addWidget(self.appr_table, 1)

        self.appr_detail_frame = QFrame()
        self.appr_detail_frame.setObjectName("taskStatsFrame")
        adf_layout = QVBoxLayout(self.appr_detail_frame)
        adf_layout.setContentsMargins(10, 8, 10, 8)
        adf_layout.setSpacing(6)

        self.appr_detail_text = QLabel("Select an approval item above to inspect.")
        self.appr_detail_text.setWordWrap(True)
        adf_layout.addWidget(self.appr_detail_text)

        appr_action_row = QHBoxLayout()
        self.btn_approve = QPushButton("✓ Authorize Action")
        self.btn_approve.setStyleSheet("background: #2d6a4f; color: #ffffff; font-weight: bold; padding: 6px 14px;")
        self.btn_approve.clicked.connect(lambda: self.resolve_approval(True))
        appr_action_row.addWidget(self.btn_approve)

        self.btn_reject = QPushButton("✗ Reject Action")
        self.btn_reject.setStyleSheet("background: #9b2226; color: #ffffff; font-weight: bold; padding: 6px 14px;")
        self.btn_reject.clicked.connect(lambda: self.resolve_approval(False))
        appr_action_row.addWidget(self.btn_reject)

        self.btn_escalate = QPushButton("⚡ Escalate / Flag")
        self.btn_escalate.clicked.connect(self.escalate_approval)
        appr_action_row.addWidget(self.btn_escalate)

        appr_action_row.addStretch()
        adf_layout.addLayout(appr_action_row)

        appr_layout.addWidget(self.appr_detail_frame)
        self.tabs.addTab(self.tab_approvals, "🛡️ Approvals & Governance")

        # Tab 4: Return Report
        self.tab_report = QWidget()
        rep_layout = QHBoxLayout(self.tab_report)
        rep_layout.setContentsMargins(6, 6, 6, 6)
        rep_layout.setSpacing(8)

        rep_left = QVBoxLayout()
        rep_left.addWidget(QLabel("<b>📜 Return Reports Archive:</b>"))
        self.rep_tree = QTreeWidget()
        self.rep_tree.setColumnCount(3)
        self.rep_tree.setHeaderLabels(["Report ID", "Timestamp", "Mode"])
        self.rep_tree.header().setSectionResizeMode(1, QHeaderView.Stretch)
        self.rep_tree.itemSelectionChanged.connect(self.display_selected_report)
        rep_left.addWidget(self.rep_tree, 1)
        rep_layout.addLayout(rep_left, 1)

        self.report_browser = QTextBrowser()
        self.report_browser.setOpenExternalLinks(True)
        self.report_browser.setObjectName("consoleOutputArea")
        rep_layout.addWidget(self.report_browser, 2)

        self.tabs.addTab(self.tab_report, "📜 Absence Return Report")

        layout.addWidget(self.tabs, 1)

        self.update_mode_ui()
        self.refresh_all()

    def set_mode(self, mode: OperatorMode):
        self.state.set_operator_mode(mode)
        self.update_mode_ui()
        self.refresh_all()

    def update_mode_ui(self):
        m = self.state.operator_mode
        if m == OperatorMode.GREEN:
            self.mode_desc.setText("<b>Status:</b> 🟢 <span style='color: #4cd964;'>Operator Active</span> — Autonomous generation active with standard human oversight on protected actions.")
        elif m == OperatorMode.YELLOW:
            self.mode_desc.setText("<b>Status:</b> 🟡 <span style='color: #ffd60a;'>Limited Physical Capacity</span> — System strictly prioritizes non-physical software work. Physical tasks (BLUE/ORANGE/RED) automatically defer to the Delegation Queue.")
        else:
            self.mode_desc.setText("<b>Status:</b> 🔴 <span style='color: #ff453a;'>Operator Unavailable (Continuity Mode)</span> — Safe background operations only. No destructive actions. Full Return Report will be synthesized upon return.")

    def refresh_all(self):
        self.populate_plan_tree()
        self.populate_blockers()
        self.populate_approvals()
        self.populate_reports()

    def populate_plan_tree(self):
        self.task_tree.clear()
        categories = [
            (TaskCategory.MUST_DO, "🔴 MUST DO (Critical Operations)"),
            (TaskCategory.REVENUE, "💰 REVENUE (Direct Value Generation)"),
            (TaskCategory.BIZ_DEV, "🚀 BUSINESS DEVELOPMENT (Future Growth)"),
            (TaskCategory.MAINTENANCE, "🔧 MAINTENANCE (Infrastructure & Hygiene)"),
            (TaskCategory.RESEARCH, "🔬 RESEARCH (Targeted Investigation)"),
            (TaskCategory.OPTIONAL, "☕ OPTIONAL (Low Priority Backlog)"),
        ]

        for cat, cat_title in categories:
            cat_item = QTreeWidgetItem([cat_title, "", "", "", "", "", ""])
            cat_item.setExpanded(True)
            self.task_tree.addTopLevelItem(cat_item)

            tasks_in_cat = [t for t in self.state.tasks.values() if t.category == cat]
            for t in tasks_in_cat:
                cost_str = t.physical_cost.value.split()[0] + " " + t.physical_cost.name
                item = QTreeWidgetItem([
                    f"{t.id}: {t.title}",
                    t.project_id,
                    cost_str,
                    t.model_level.name,
                    t.worker_role,
                    t.status.value,
                    f"${t.cost_est:.2f}"
                ])
                item.setData(0, Qt.UserRole, t.id)
                cat_item.addChild(item)

    def get_selected_task_id(self):
        item = self.task_tree.currentItem()
        if not item:
            return None
        return item.data(0, Qt.UserRole)

    def update_task_status_label(self):
        tid = self.get_selected_task_id()
        if tid and tid in self.state.tasks:
            t = self.state.tasks[tid]
            self.task_status_lbl.setText(f"Selected: {t.title} [{t.status.value}]")
        else:
            self.task_status_lbl.setText("")

    def advance_selected_task(self):
        tid = self.get_selected_task_id()
        if tid:
            msg = self.state.advance_task(tid)
            self.refresh_all()
            self.task_status_lbl.setText(msg)

    def view_selected_task(self):
        tid = self.get_selected_task_id()
        if tid and tid in self.state.tasks:
            t = self.state.tasks[tid]
            self.portal.log("INFO", "TASK_VIEW", f"{t.id} - {t.title}: {t.output}")
            self.portal.status(f"Task: {t.title} - Status: {t.status.value}")

    def step_next_task(self):
        for t in self.state.tasks.values():
            if t.status in (TaskStatus.PENDING, TaskStatus.IN_PROGRESS):
                msg = self.state.advance_task(t.id)
                self.refresh_all()
                self.task_status_lbl.setText(msg)
                return
        self.portal.status("No pending or active tasks to step.")

    def generate_plan(self):
        self.state.generate_daily_plan()
        self.refresh_all()
        self.portal.status("Daily work plan updated.")

    def generate_report(self):
        rep = self.state.generate_return_report()
        self.refresh_all()
        self.tabs.setCurrentIndex(3)
        self.portal.status(f"Generated Return Report {rep.id}.")

    def populate_blockers(self):
        self.blocker_list.clear()
        for b in self.state.blockers.values():
            cost_tag = b.physical_cost.value.split()[0] + " " + b.physical_cost.name
            item = QTreeWidgetItem([b.id, b.task_title, cost_tag, b.est_time])
            item.setData(0, Qt.UserRole, b.id)
            self.blocker_list.addTopLevelItem(item)

        if self.blocker_list.topLevelItemCount() > 0:
            self.blocker_list.setCurrentItem(self.blocker_list.topLevelItem(0))
            self.update_delegation_sheet()

    def update_delegation_sheet(self):
        item = self.blocker_list.currentItem()
        if not item:
            return
        bid = item.data(0, Qt.UserRole)
        if not bid or bid not in self.state.blockers:
            return

        b = self.state.blockers[bid]
        self.del_task_title.setText(f"Task: {b.task_title}")
        self.del_demand_badge.setText(f"<b>Physical Demand:</b> {b.physical_cost.value} &nbsp;|&nbsp; <b>Est Time:</b> {b.est_time} &nbsp;|&nbsp; <b>Status:</b> {b.delegation_status}")
        self.del_why.setText(f"<b>Why Required:</b> {b.description}")
        self.del_tools.setText(f"<b>Tools & Equipment Needed:</b> {b.tools_required}")
        self.del_safety.setText(f"<b>Safety Considerations:</b> <span style='color: #ff9f0a;'>{b.safety_notes}</span>")
        self.del_value.setText(f"<b>Business Value:</b> {b.business_value} &nbsp;|&nbsp; <b>Deadline:</b> {b.deadline}")
        self.btn_resolve_blk.setEnabled(b.delegation_status != "Completed by Helper")

    def resolve_selected_blocker(self):
        item = self.blocker_list.currentItem()
        if not item:
            return
        bid = item.data(0, Qt.UserRole)
        if bid:
            msg = self.state.resolve_physical_blocker(bid)
            self.refresh_all()
            self.portal.status(msg)

    def populate_approvals(self):
        self.appr_table.setRowCount(0)
        for row, a in enumerate(self.state.approvals.values()):
            self.appr_table.insertRow(row)
            self.appr_table.setItem(row, 0, QTableWidgetItem(a.id))
            self.appr_table.setItem(row, 1, QTableWidgetItem(a.task_id))
            self.appr_table.setItem(row, 2, QTableWidgetItem(a.description))
            self.appr_table.setItem(row, 3, QTableWidgetItem(f"[{a.risk_level}]"))
            self.appr_table.setItem(row, 4, QTableWidgetItem(a.requested_by))
            self.appr_table.setItem(row, 5, QTableWidgetItem(a.status))

        if self.appr_table.rowCount() > 0:
            self.appr_table.selectRow(0)
            self.update_approval_detail()

    def update_approval_detail(self):
        row = self.appr_table.currentRow()
        if row < 0:
            return
        aid_item = self.appr_table.item(row, 0)
        if not aid_item:
            return
        aid = aid_item.text()
        if aid in self.state.approvals:
            a = self.state.approvals[aid]
            self.appr_detail_text.setText(
                f"<b>Action Request [{a.id}]:</b> {a.description}<br>"
                f"<b>Risk Level:</b> {a.risk_level} &nbsp;|&nbsp; <b>Requested By:</b> {a.requested_by} &nbsp;|&nbsp; <b>Time:</b> {a.timestamp}<br>"
                f"<b>Execution Details:</b> {a.details}<br>"
                f"<b>Current Authorization Status:</b> <b>{a.status}</b>"
            )
            is_pending = (a.status == "PENDING")
            self.btn_approve.setEnabled(is_pending)
            self.btn_reject.setEnabled(is_pending)

    def resolve_approval(self, approved: bool):
        row = self.appr_table.currentRow()
        if row < 0:
            return
        aid_item = self.appr_table.item(row, 0)
        if not aid_item:
            return
        aid = aid_item.text()
        msg = self.state.resolve_approval(aid, approved)
        self.refresh_all()
        self.portal.status(msg)

    def escalate_approval(self):
        row = self.appr_table.currentRow()
        if row < 0:
            return
        aid_item = self.appr_table.item(row, 0)
        if not aid_item:
            return
        aid = aid_item.text()
        self.portal.log("WARN", "APPROVAL_ESCALATE", f"Approval {aid} flagged for operator review / modification.")
        self.portal.status(f"Approval {aid} flagged for deeper review.")

    def populate_reports(self):
        self.rep_tree.clear()
        for r in self.state.return_reports:
            item = QTreeWidgetItem([r.id, r.timestamp, r.mode_during_absence])
            item.setData(0, Qt.UserRole, r.id)
            self.rep_tree.addTopLevelItem(item)

        if self.rep_tree.topLevelItemCount() > 0:
            self.rep_tree.setCurrentItem(self.rep_tree.topLevelItem(0))
            self.display_selected_report()

    def display_selected_report(self):
        item = self.rep_tree.currentItem()
        if not item:
            return
        rid = item.data(0, Qt.UserRole)
        report = next((r for r in self.state.return_reports if r.id == rid), None)
        if not report:
            return

        html = f"""
        <h2 style='color: #4da6ff; margin-bottom: 4px;'>Business Autonomy Return Report ({report.id})</h2>
        <div style='color: #888; margin-bottom: 12px;'>Generated: {report.timestamp} | Mode: <b>{report.mode_during_absence}</b> | Duration: {report.duration_hours} hrs</div>
        
        <h3 style='color: #4cd964;'>✓ What Completed While Away:</h3>
        <ul>{''.join(f'<li>{t}</li>' for t in report.tasks_completed)}</ul>

        <h3 style='color: #ffd60a;'>💰 Financial & Resource Summary:</h3>
        <ul>
            <li><b>Revenue Generated:</b> <span style='color: #4cd964;'>+${report.revenue_generated:.2f}</span></li>
            <li><b>Total Model Compute Cost:</b> <span style='color: #ff453a;'>-${report.costs_incurred:.2f}</span></li>
            <li><b>Net Economic Value:</b> <span style='color: #4da6ff; font-weight: bold;'>+${report.revenue_generated - report.costs_incurred:.2f}</span></li>
        </ul>

        <h3 style='color: #ff9f0a;'>🚧 Physical Assistance Queue:</h3>
        <ul>{''.join(f'<li>{item}</li>' for item in report.human_action_items)}</ul>

        <h3 style='color: #a0a0ff;'>🎯 Recommended Next Actions:</h3>
        <ol>{''.join(f'<li>{item}</li>' for item in report.next_recommended_plan)}</ol>
        """
        self.report_browser.setHtml(html)


class BusinessSupervisorApplication(PortalApplication):
    application_id = "autonomy"
    display_name = "Business Autonomy"
    icon = "👑"
    default_size = (780, 520)
    layout_policy = WindowLayoutPolicy(min_width=480, min_height=320)
    behavior_policy = WindowBehaviorPolicy()

    def create_widget(self, portal):
        return BusinessSupervisorWidget(portal)


# ---------------------------------------------------------------------------
# Business Database Application
# ---------------------------------------------------------------------------

class BusinessDatabaseWidget(QWidget):
    """Structured relational explorer for Projects, Master Tasks, Revenue Engine, and Persistent Memory."""
    def __init__(self, portal):
        super().__init__()
        self.portal = portal
        self.state: BusinessStateEngine = getattr(portal, "business_state", None)
        if not self.state:
            self.state = BusinessStateEngine(portal)
            portal.business_state = self.state

        layout = QVBoxLayout(self)
        layout.setContentsMargins(8, 8, 8, 8)
        layout.setSpacing(6)

        self.tabs = QTabWidget()
        self.tabs.setObjectName("databaseTabs")

        # Tab 1: Projects
        self.tab_projects = QWidget()
        proj_layout = QVBoxLayout(self.tab_projects)
        proj_layout.setContentsMargins(6, 6, 6, 6)
        proj_layout.setSpacing(6)

        self.proj_table = QTableWidget()
        self.proj_table.setColumnCount(8)
        self.proj_table.setHorizontalHeaderLabels(["ID", "Project Name", "Objective", "Status", "Priority", "Revenue Potential", "Physical Req", "Next Action"])
        self.proj_table.horizontalHeader().setSectionResizeMode(1, QHeaderView.ResizeToContents)
        self.proj_table.horizontalHeader().setSectionResizeMode(2, QHeaderView.Stretch)
        proj_layout.addWidget(self.proj_table)
        self.tabs.addTab(self.tab_projects, "🏢 Projects")

        # Tab 2: Master Tasks
        self.tab_tasks = QWidget()
        tasks_layout = QVBoxLayout(self.tab_tasks)
        tasks_layout.setContentsMargins(6, 6, 6, 6)
        tasks_layout.setSpacing(6)

        t_filter_row = QHBoxLayout()
        t_filter_row.addWidget(QLabel("Physical Filter:"))
        self.cost_filter_combo = QComboBox()
        self.cost_filter_combo.addItems(["All Tasks", "🟢 Software-Only (Green)", "🔵+ Physical Required (Blue/Orange/Red)"])
        self.cost_filter_combo.currentTextChanged.connect(self.populate_tasks)
        t_filter_row.addWidget(self.cost_filter_combo)

        t_filter_row.addWidget(QLabel("Category:"))
        self.cat_filter_combo = QComboBox()
        self.cat_filter_combo.addItems(["All Categories", "MUST DO", "REVENUE", "BIZ DEV", "MAINTENANCE", "RESEARCH", "OPTIONAL"])
        self.cat_filter_combo.currentTextChanged.connect(self.populate_tasks)
        t_filter_row.addWidget(self.cat_filter_combo)

        self.task_search = QLineEdit()
        self.task_search.setPlaceholderText("🔍 Search tasks...")
        self.task_search.textChanged.connect(self.populate_tasks)
        t_filter_row.addWidget(self.task_search, 1)

        tasks_layout.addLayout(t_filter_row)

        self.master_task_table = QTableWidget()
        self.master_task_table.setColumnCount(9)
        self.master_task_table.setHorizontalHeaderLabels(["ID", "Project", "Title", "Category", "Physical Cost", "Model Tier", "Worker", "Status", "QA Status"])
        self.master_task_table.horizontalHeader().setSectionResizeMode(2, QHeaderView.Stretch)
        tasks_layout.addWidget(self.master_task_table)
        self.tabs.addTab(self.tab_tasks, "📋 Master Tasks")

        # Tab 3: Revenue Engine
        self.tab_revenue = QWidget()
        rev_layout = QVBoxLayout(self.tab_revenue)
        rev_layout.setContentsMargins(6, 6, 6, 6)
        rev_layout.setSpacing(6)

        info_box = QLabel("<b>Revenue Formula:</b> ROI Score = (Est Value × Probability × Reusability) / Effort &nbsp;&nbsp;|&nbsp;&nbsp; <i>Prioritize high ROI non-physical digital products</i>")
        info_box.setObjectName("pathLabel")
        rev_layout.addWidget(info_box)

        self.rev_table = QTableWidget()
        self.rev_table.setColumnCount(9)
        self.rev_table.setHorizontalHeaderLabels(["ID", "Opportunity", "Category", "Target Customer", "Est Value", "Effort", "Prob", "ROI Score", "Physical Req"])
        self.rev_table.horizontalHeader().setSectionResizeMode(1, QHeaderView.Stretch)
        rev_layout.addWidget(self.rev_table)
        self.tabs.addTab(self.tab_revenue, "💰 Revenue Engine")

        # Tab 4: Persistent Memory
        self.tab_memory = QWidget()
        mem_layout = QVBoxLayout(self.tab_memory)
        mem_layout.setContentsMargins(6, 6, 6, 6)
        mem_layout.setSpacing(6)

        mem_layout.addWidget(QLabel("<b>📜 Business Decisions Ledger:</b>"))
        self.dec_table = QTableWidget()
        self.dec_table.setColumnCount(5)
        self.dec_table.setHorizontalHeaderLabels(["ID", "Date", "Decision", "Reason", "Consequence"])
        self.dec_table.horizontalHeader().setSectionResizeMode(2, QHeaderView.Stretch)
        mem_layout.addWidget(self.dec_table, 1)

        mem_layout.addWidget(QLabel("<b>💡 Lessons Learned & Operational Rules:</b>"))
        self.les_table = QTableWidget()
        self.les_table.setColumnCount(5)
        self.les_table.setHorizontalHeaderLabels(["ID", "Date", "Failure Event", "Root Cause", "Future Operating Rule"])
        self.les_table.horizontalHeader().setSectionResizeMode(4, QHeaderView.Stretch)
        mem_layout.addWidget(self.les_table, 1)

        self.tabs.addTab(self.tab_memory, "🧠 Persistent Memory")

        layout.addWidget(self.tabs)
        self.refresh_all()

    def refresh_all(self):
        self.populate_projects()
        self.populate_tasks()
        self.populate_revenue()
        self.populate_memory()

    def populate_projects(self):
        self.proj_table.setRowCount(0)
        for row, p in enumerate(self.state.projects.values()):
            self.proj_table.insertRow(row)
            self.proj_table.setItem(row, 0, QTableWidgetItem(p.id))
            self.proj_table.setItem(row, 1, QTableWidgetItem(p.name))
            self.proj_table.setItem(row, 2, QTableWidgetItem(p.objective))
            self.proj_table.setItem(row, 3, QTableWidgetItem(p.status))
            self.proj_table.setItem(row, 4, QTableWidgetItem(p.priority))
            self.proj_table.setItem(row, 5, QTableWidgetItem(p.revenue_potential))
            self.proj_table.setItem(row, 6, QTableWidgetItem(p.physical_req))
            self.proj_table.setItem(row, 7, QTableWidgetItem(p.next_action))

    def populate_tasks(self):
        cost_filter = self.cost_filter_combo.currentText()
        cat_filter = self.cat_filter_combo.currentText()
        q = self.task_search.text().lower()

        self.master_task_table.setRowCount(0)
        row = 0
        for t in self.state.tasks.values():
            if "Software-Only" in cost_filter and t.physical_cost != PhysicalCost.GREEN:
                continue
            if "Physical Required" in cost_filter and t.physical_cost == PhysicalCost.GREEN:
                continue
            if cat_filter != "All Categories" and cat_filter not in t.category.name:
                continue
            if q and q not in t.title.lower() and q not in t.id.lower() and q not in t.worker_role.lower():
                continue

            self.master_task_table.insertRow(row)
            self.master_task_table.setItem(row, 0, QTableWidgetItem(t.id))
            self.master_task_table.setItem(row, 1, QTableWidgetItem(t.project_id))
            self.master_task_table.setItem(row, 2, QTableWidgetItem(t.title))
            self.master_task_table.setItem(row, 3, QTableWidgetItem(t.category.name))
            self.master_task_table.setItem(row, 4, QTableWidgetItem(t.physical_cost.name))
            self.master_task_table.setItem(row, 5, QTableWidgetItem(t.model_level.name))
            self.master_task_table.setItem(row, 6, QTableWidgetItem(t.worker_role))
            self.master_task_table.setItem(row, 7, QTableWidgetItem(t.status.value))
            self.master_task_table.setItem(row, 8, QTableWidgetItem(t.qa_status))
            row += 1

    def populate_revenue(self):
        self.rev_table.setRowCount(0)
        for row, r in enumerate(self.state.revenue_opportunities.values()):
            self.rev_table.insertRow(row)
            self.rev_table.setItem(row, 0, QTableWidgetItem(r.id))
            self.rev_table.setItem(row, 1, QTableWidgetItem(r.title))
            self.rev_table.setItem(row, 2, QTableWidgetItem(r.category))
            self.rev_table.setItem(row, 3, QTableWidgetItem(r.customer_target))
            self.rev_table.setItem(row, 4, QTableWidgetItem(f"${r.estimated_value:,.2f}"))
            self.rev_table.setItem(row, 5, QTableWidgetItem(f"{r.effort_score}/10"))
            self.rev_table.setItem(row, 6, QTableWidgetItem(f"{int(r.probability*100)}%"))
            self.rev_table.setItem(row, 7, QTableWidgetItem(f"{r.roi_score:.1f}"))
            self.rev_table.setItem(row, 8, QTableWidgetItem(r.physical_requirement.name))

    def populate_memory(self):
        self.dec_table.setRowCount(0)
        for row, d in enumerate(self.state.decisions):
            self.dec_table.insertRow(row)
            self.dec_table.setItem(row, 0, QTableWidgetItem(d.id))
            self.dec_table.setItem(row, 1, QTableWidgetItem(d.date_str))
            self.dec_table.setItem(row, 2, QTableWidgetItem(d.decision))
            self.dec_table.setItem(row, 3, QTableWidgetItem(d.reason))
            self.dec_table.setItem(row, 4, QTableWidgetItem(d.consequence))

        self.les_table.setRowCount(0)
        for row, l in enumerate(self.state.lessons):
            self.les_table.insertRow(row)
            self.les_table.setItem(row, 0, QTableWidgetItem(l.id))
            self.les_table.setItem(row, 1, QTableWidgetItem(l.date_str))
            self.les_table.setItem(row, 2, QTableWidgetItem(l.failure))
            self.les_table.setItem(row, 3, QTableWidgetItem(l.root_cause))
            self.les_table.setItem(row, 4, QTableWidgetItem(l.future_rule))


class BusinessDatabaseApplication(PortalApplication):
    application_id = "bizdb"
    display_name = "Business Database"
    icon = "🏢"
    default_size = (740, 480)
    layout_policy = WindowLayoutPolicy(min_width=460, min_height=300)
    behavior_policy = WindowBehaviorPolicy()

    def create_widget(self, portal):
        return BusinessDatabaseWidget(portal)


# ---------------------------------------------------------------------------
# Model Router Application
# ---------------------------------------------------------------------------

class ModelRouterWidget(QWidget):
    """Model tier routing allocation, worker fleet live monitoring, and token/cost analytics."""
    def __init__(self, portal):
        super().__init__()
        self.portal = portal
        self.state: BusinessStateEngine = getattr(portal, "business_state", None)
        if not self.state:
            self.state = BusinessStateEngine(portal)
            portal.business_state = self.state

        layout = QVBoxLayout(self)
        layout.setContentsMargins(8, 8, 8, 8)
        layout.setSpacing(6)

        stats_frame = QFrame()
        stats_frame.setObjectName("taskStatsFrame")
        stats_layout = QHBoxLayout(stats_frame)
        stats_layout.setContentsMargins(10, 8, 10, 8)
        stats_layout.setSpacing(16)

        self.lbl_tokens = QLabel(f"Tokens: {self.state.total_tokens:,}")
        self.lbl_tokens.setObjectName("taskMetricBadge")
        stats_layout.addWidget(self.lbl_tokens)

        self.lbl_cost = QLabel(f"Total AI Spend: ${self.state.total_cost:.2f}")
        self.lbl_cost.setObjectName("taskMetricBadge")
        stats_layout.addWidget(self.lbl_cost)

        self.lbl_val = QLabel(f"Value Created: ${self.state.value_generated:.2f}")
        self.lbl_val.setObjectName("taskMetricBadge")
        stats_layout.addWidget(self.lbl_val)

        stats_layout.addStretch()

        self.lbl_ratio = QLabel("ROI Multiplier: 1,154x | Local Compute: 86%")
        self.lbl_ratio.setObjectName("pathLabel")
        stats_layout.addWidget(self.lbl_ratio)

        layout.addWidget(stats_frame)

        layout.addWidget(QLabel("<b>Model Tier Routing Matrix (Policy: Cheapest Reliable Model):</b>"))
        self.tier_table = QTableWidget()
        self.tier_table.setColumnCount(5)
        self.tier_table.setHorizontalHeaderLabels(["Tier", "Model Runtime / Engine", "Token Cost", "Target Workloads", "Routing Policy"])
        self.tier_table.horizontalHeader().setSectionResizeMode(3, QHeaderView.Stretch)
        layout.addWidget(self.tier_table, 1)

        layout.addWidget(QLabel("<b>Specialized Worker Fleet:</b>"))
        self.worker_table = QTableWidget()
        self.worker_table.setColumnCount(7)
        self.worker_table.setHorizontalHeaderLabels(["Worker ID", "Role Name", "Default Tier", "Status", "Current Assignment", "Tasks Done", "Tokens Burned"])
        self.worker_table.horizontalHeader().setSectionResizeMode(4, QHeaderView.Stretch)
        layout.addWidget(self.worker_table, 1)

        self.refresh_all()

    def refresh_all(self):
        self.lbl_tokens.setText(f"Tokens: {self.state.total_tokens:,}")
        self.lbl_cost.setText(f"Total AI Spend: ${self.state.total_cost:.2f}")
        self.lbl_val.setText(f"Value Created: ${self.state.value_generated:.2f}")

        # Tiers
        tiers = [
            ("Level 0", "Deterministic Code / Scripts", "$0.00 / 0 Tokens", "Renaming, validation, regex, backups, tests", "Mandatory First Filter"),
            ("Level 1", "Local Ollama / Local GPU Model", "$0.00 (Local Compute)", "Classification, tags, summarization, triage", "Default Autonomous Worker"),
            ("Level 2", "Cheap Cloud Model (e.g. Flash/Haiku)", "$0.0005 / 1k Tokens", "Deep research, complex coding, multi-step analysis", "Escalate if Local Inadequate"),
            ("Level 3", "High-End Frontier Model (e.g. Opus/Pro)", "$0.0150 / 1k Tokens", "Strategic decisions, complex architecture, heavy recovery", "Operator Authorization Only"),
        ]
        self.tier_table.setRowCount(len(tiers))
        for row, (tier, engine, cost, target, policy) in enumerate(tiers):
            self.tier_table.setItem(row, 0, QTableWidgetItem(tier))
            self.tier_table.setItem(row, 1, QTableWidgetItem(engine))
            self.tier_table.setItem(row, 2, QTableWidgetItem(cost))
            self.tier_table.setItem(row, 3, QTableWidgetItem(target))
            self.tier_table.setItem(row, 4, QTableWidgetItem(policy))

        # Workers
        self.worker_table.setRowCount(0)
        for row, w in enumerate(self.state.workers.values()):
            self.worker_table.insertRow(row)
            self.worker_table.setItem(row, 0, QTableWidgetItem(w.id))
            self.worker_table.setItem(row, 1, QTableWidgetItem(w.role_name))
            self.worker_table.setItem(row, 2, QTableWidgetItem(w.default_tier.name))
            self.worker_table.setItem(row, 3, QTableWidgetItem(w.status))
            self.worker_table.setItem(row, 4, QTableWidgetItem(w.current_task))
            self.worker_table.setItem(row, 5, QTableWidgetItem(str(w.tasks_completed)))
            self.worker_table.setItem(row, 6, QTableWidgetItem(f"{w.tokens_used:,}"))


class ModelRouterApplication(PortalApplication):
    application_id = "agents"
    display_name = "Model Router & Workers"
    icon = "🤖"
    default_size = (700, 460)
    layout_policy = WindowLayoutPolicy(min_width=440, min_height=280)
    behavior_policy = WindowBehaviorPolicy()

    def create_widget(self, portal):
        return ModelRouterWidget(portal)



# ---------------------------------------------------------------------------
# Internal Window Composite Elements & Policy Engine
# ---------------------------------------------------------------------------

class WindowElement(QFrame):
    """Base UI element component for top-down derived window structures."""
    def __init__(self, parent=None):
        super().__init__(parent)


class ControlButtonGroupElement(WindowElement):
    """Derived window control button group governed by WindowLayoutPolicy."""
    def __init__(self, layout_policy: WindowLayoutPolicy, parent=None):
        super().__init__(parent)
        self.layout_policy = layout_policy
        self.layout = QHBoxLayout(self)
        self.layout.setContentsMargins(0, 0, 0, 0)
        self.layout.setSpacing(0)

        self.min_button = QPushButton("—") if layout_policy.show_minimize else None
        self.max_button = QPushButton("□") if layout_policy.show_maximize else None
        self.close_button = QPushButton("×") if layout_policy.show_close else None

        for b in (self.min_button, self.max_button, self.close_button):
            if b:
                b.setFixedSize(34, layout_policy.title_height)
                b.setFocusPolicy(Qt.NoFocus)
                b.setCursor(Qt.PointingHandCursor)
                self.layout.addWidget(b)


class TitleBarElement(WindowElement):
    """Top-down derived titlebar widget structured cleanly via layout containers."""
    def __init__(self, app, layout_policy: WindowLayoutPolicy, parent=None):
        super().__init__(parent)
        self.setObjectName("titlebar")
        self.layout_policy = layout_policy
        self.setFixedHeight(layout_policy.title_height)

        self.layout = QHBoxLayout(self)
        self.layout.setContentsMargins(10, 0, 0, 0)
        self.layout.setSpacing(8)

        text = f"{app.icon}  {app.display_name}" if (app and layout_policy.show_icon) else (app.display_name if app else "Task")
        self.title_label = QLabel(text)
        self.title_label.setObjectName("windowTitle")
        self.title_label.setAlignment(Qt.AlignVCenter | Qt.AlignLeft)
        self.layout.addWidget(self.title_label, 1)

        self.controls = ControlButtonGroupElement(layout_policy, self)
        self.layout.addWidget(self.controls, 0)

    def set_title(self, text: str):
        self.title_label.setText(text)

    def mousePressEvent(self, event):
        if event.button() == Qt.LeftButton:
            child = self.childAt(event.position().toPoint())
            if not isinstance(child, QPushButton):
                win = self.parent()
                if isinstance(win, PortalWindow):
                    win.start_window_drag(event.globalPosition().toPoint())
                    event.accept()
                    return
        super().mousePressEvent(event)

    def mouseMoveEvent(self, event):
        win = self.parent()
        if isinstance(win, PortalWindow) and win.dragging:
            win.continue_window_drag(event.globalPosition().toPoint())
            event.accept()
            return
        super().mouseMoveEvent(event)

    def mouseReleaseEvent(self, event):
        win = self.parent()
        if isinstance(win, PortalWindow) and win.dragging:
            win.end_window_drag(event.globalPosition().toPoint())
            event.accept()
            return
        super().mouseReleaseEvent(event)

    def mouseDoubleClickEvent(self, event):
        if event.button() == Qt.LeftButton:
            win = self.parent()
            if isinstance(win, PortalWindow) and win.behavior_policy.allow_maximize:
                win.requestMaximize.emit(win)
                event.accept()
                return
        super().mouseDoubleClickEvent(event)


class PaneHeader(QFrame):
    """Header bar for individual panes inside a PortalWindow."""
    undockRequested = Signal(object)
    closeRequested = Signal(object)
    splitRequested = Signal(object, object, object)  # pane, orientation, position

    def __init__(self, pane, parent=None):
        super().__init__(parent)
        self.pane = pane
        self.setObjectName("paneHeader")
        self.setFixedHeight(26)

        self.layout = QHBoxLayout(self)
        self.layout.setContentsMargins(8, 0, 4, 0)
        self.layout.setSpacing(4)

        self.title_label = QLabel()
        self.title_label.setObjectName("paneTitleLabel")
        self.title_label.setAlignment(Qt.AlignVCenter | Qt.AlignLeft)
        self.layout.addWidget(self.title_label, 1)

        # Quick-split buttons directly on pane header
        self.split_h_btn = QPushButton("◧")
        self.split_h_btn.setObjectName("paneSplitHButton")
        self.split_h_btn.setToolTip("Split pane right (vertical divider)")
        self.split_h_btn.setFixedSize(22, 20)
        self.split_h_btn.setFocusPolicy(Qt.NoFocus)
        self.split_h_btn.setCursor(Qt.PointingHandCursor)
        self.split_h_btn.clicked.connect(lambda: self.splitRequested.emit(self.pane, Qt.Horizontal, "after"))
        self.layout.addWidget(self.split_h_btn)

        self.split_v_btn = QPushButton("⬓")
        self.split_v_btn.setObjectName("paneSplitVButton")
        self.split_v_btn.setToolTip("Split pane down (horizontal divider)")
        self.split_v_btn.setFixedSize(22, 20)
        self.split_v_btn.setFocusPolicy(Qt.NoFocus)
        self.split_v_btn.setCursor(Qt.PointingHandCursor)
        self.split_v_btn.clicked.connect(lambda: self.splitRequested.emit(self.pane, Qt.Vertical, "after"))
        self.layout.addWidget(self.split_v_btn)

        self.undock_btn = QPushButton("⇱")
        self.undock_btn.setObjectName("paneUndockButton")
        self.undock_btn.setToolTip("Undock into independent window")
        self.undock_btn.setFixedSize(22, 20)
        self.undock_btn.setFocusPolicy(Qt.NoFocus)
        self.undock_btn.setCursor(Qt.PointingHandCursor)
        self.undock_btn.clicked.connect(lambda: self.undockRequested.emit(self.pane))
        self.layout.addWidget(self.undock_btn)

        self.close_btn = QPushButton("×")
        self.close_btn.setObjectName("paneCloseButton")
        self.close_btn.setToolTip("Close this pane")
        self.close_btn.setFixedSize(22, 20)
        self.close_btn.setFocusPolicy(Qt.NoFocus)
        self.close_btn.setCursor(Qt.PointingHandCursor)
        self.close_btn.clicked.connect(lambda: self.closeRequested.emit(self.pane))
        self.layout.addWidget(self.close_btn)

        self.update_title()

    def update_title(self):
        if self.pane.app:
            self.title_label.setText(f"{self.pane.app.icon}  {self.pane.app.display_name}")
        else:
            self.title_label.setText("◫  New Split Slot")


class PanePlaceholder(QFrame):
    """Placeholder view when a split slot is waiting for an app or drop."""
    appSelected = Signal(str)

    def __init__(self, portal, parent=None):
        super().__init__(parent)
        self.portal = portal
        self.setObjectName("panePlaceholder")

        layout = QVBoxLayout(self)
        layout.setAlignment(Qt.AlignCenter)
        layout.setSpacing(10)

        icon_lbl = QLabel("◫")
        icon_lbl.setObjectName("placeholderIcon")
        icon_lbl.setAlignment(Qt.AlignCenter)
        layout.addWidget(icon_lbl)

        desc = QLabel("Drag a window here or launch an app:")
        desc.setObjectName("placeholderText")
        desc.setAlignment(Qt.AlignCenter)
        layout.addWidget(desc)

        btn_grid = QGridLayout()
        btn_grid.setSpacing(6)

        app_list = [
            ("autonomy", "👑 Autonomy"),
            ("bizdb", "🏢 Biz DB"),
            ("agents", "🤖 Workers"),
            ("tasks", "▤ Tasks"),
            ("console", "⌨ Console"),
            ("files", "📁 Files"),
            ("grid", "▦ Grid"),
            ("settings", "⚙ Settings"),
            ("about", "ⓘ About"),
            ("home", "⌂ Home"),
        ]

        for idx, (app_id, label) in enumerate(app_list):
            btn = QPushButton(label)
            btn.setObjectName("placeholderAppBtn")
            btn.setCursor(Qt.PointingHandCursor)
            btn.clicked.connect(lambda chk=False, aid=app_id: self.appSelected.emit(aid))
            btn_grid.addWidget(btn, idx // 3, idx % 3)

        layout.addLayout(btn_grid)


class PortalSplitter(QSplitter):
    """Custom splitter ensuring fluid and consistent resizing across arbitrarily nested multi-split hierarchies."""
    def __init__(self, orientation=Qt.Horizontal, parent=None):
        super().__init__(orientation, parent)
        self.setObjectName("windowSplitter")
        self.setChildrenCollapsible(False)
        self.setHandleWidth(4)
        self.setSizePolicy(QSizePolicy.Expanding, QSizePolicy.Expanding)

    def minimumSizeHint(self):
        return QSize(40, 40)

    def addWidget(self, widget):
        super().addWidget(widget)
        self.setStretchFactor(self.count() - 1, 1)

    def insertWidget(self, index, widget):
        super().insertWidget(index, widget)
        self.setStretchFactor(index, 1)


class WindowPane(QFrame):
    """Individual application pane inside a PortalWindow splitter hierarchy."""
    undockRequested = Signal(object)
    closeRequested = Signal(object)
    splitRequested = Signal(object, object, object)

    def __init__(self, portal, app=None, content_widget=None, parent=None):
        super().__init__(parent)
        self.portal = portal
        self.app = app
        self.setObjectName("windowPane")
        self.setSizePolicy(QSizePolicy.Expanding, QSizePolicy.Expanding)

        self.layout = QVBoxLayout(self)
        self.layout.setContentsMargins(0, 0, 0, 0)
        self.layout.setSpacing(0)

        self.header = PaneHeader(self)
        self.header.undockRequested.connect(self.undockRequested)
        self.header.closeRequested.connect(self.closeRequested)
        self.header.splitRequested.connect(self.splitRequested)
        self.layout.addWidget(self.header, 0)

        if content_widget is not None:
            self.content_widget = content_widget
        elif app is not None:
            self.content_widget = app.create_widget(portal)
        else:
            self.content_widget = PanePlaceholder(portal, self)
            self.content_widget.appSelected.connect(self.set_app)

        self.layout.addWidget(self.content_widget, 1)

    def minimumSizeHint(self):
        return QSize(40, 40)

    def get_portal_window(self):
        p = self.parent()
        while p is not None:
            if isinstance(p, PortalWindow):
                return p
            p = p.parent()
        return None

    def set_app(self, app_id):
        if app_id in self.portal.apps:
            app = self.portal.apps[app_id]
            self.app = app
            self.header.update_title()

            self.layout.removeWidget(self.content_widget)
            self.content_widget.deleteLater()
            self.content_widget = app.create_widget(self.portal)
            self.layout.addWidget(self.content_widget, 1)

            win = self.get_portal_window()
            if win:
                win.update_titlebar_and_taskbar()

    def mousePressEvent(self, event):
        win = self.get_portal_window()
        if win:
            self.portal.desktop.focus_window(win)
        super().mousePressEvent(event)


class DropPreviewOverlay(QFrame):
    """Visual feedback overlay displaying precisely where a dragged window will dock."""
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setObjectName("dropPreviewOverlay")
        self.setAttribute(Qt.WA_TransparentForMouseEvents, True)
        self.hide()

        layout = QVBoxLayout(self)
        layout.setAlignment(Qt.AlignCenter)

        self.label = QLabel("◫ Dock")
        self.label.setObjectName("dropPreviewLabel")
        self.label.setAlignment(Qt.AlignCenter)
        layout.addWidget(self.label)

    def show_preview(self, win, zone, target_pane=None):
        tb_h = win.titlebar.height() if win.titlebar and win.titlebar.isVisible() else 0
        tw = win.width()
        th = win.height()
        content_h = max(1, th - tb_h)

        if target_pane is not None and target_pane.isVisible():
            pane_pos = target_pane.mapTo(win, QPoint(0, 0))
            px = pane_pos.x()
            py = pane_pos.y()
            pw = target_pane.width()
            ph = target_pane.height()

            if zone == "left":
                self.setGeometry(px, py, max(20, pw // 2), ph)
                self.label.setText("◧  Split Left")
            elif zone == "right":
                self.setGeometry(px + pw // 2, py, max(20, pw - pw // 2), ph)
                self.label.setText("◨  Split Right")
            elif zone == "top":
                self.setGeometry(px, py, pw, max(20, ph // 2))
                self.label.setText("⬒  Split Top")
            elif zone == "bottom":
                self.setGeometry(px, py + ph // 2, pw, max(20, ph - ph // 2))
                self.label.setText("⬓  Split Bottom")
        else:
            if zone == "left":
                self.setGeometry(0, tb_h, max(20, tw // 2), content_h)
                self.label.setText("◧  Dock Left")
            elif zone == "right":
                self.setGeometry(tw // 2, tb_h, max(20, tw - tw // 2), content_h)
                self.label.setText("◨  Dock Right")
            elif zone == "top":
                self.setGeometry(0, tb_h, tw, max(20, content_h // 2))
                self.label.setText("⬒  Dock Top")
            elif zone == "bottom":
                self.setGeometry(0, tb_h + content_h // 2, tw, max(20, content_h - content_h // 2))
                self.label.setText("⬓  Dock Bottom")

        self.show()
        self.raise_()


class PortalWindow(QFrame):
    focusedChanged = Signal(object, bool)
    requestClose = Signal(object)
    requestMinimize = Signal(object)
    requestMaximize = Signal(object)

    def __init__(self, portal, app=None, geometry=None, layout_policy=None, behavior_policy=None, pane=None):
        super().__init__(portal.desktop)
        self.portal = portal
        self.app = app or (pane.app if pane else None)
        self.layout_policy = layout_policy or (getattr(self.app, "layout_policy", WindowLayoutPolicy()) if self.app else WindowLayoutPolicy())
        self.behavior_policy = behavior_policy or (getattr(self.app, "behavior_policy", WindowBehaviorPolicy()) if self.app else WindowBehaviorPolicy())

        self.state = WindowState.NORMAL
        self.geometry_state = geometry or WindowGeometry(100, 100, 520, 360)
        self.drag_offset = QPoint()
        self.drag_start_global = QPoint()
        self.drag_start_geometry = None
        self.resize_edges = set()
        self.resizing = False
        self.dragging = False

        self._last_drop_win = None
        self._last_drop_pane = None
        self._last_drop_zone = None

        self.setMouseTracking(True)
        self.setFrameShape(QFrame.NoFrame)
        self.setObjectName("portalWindow")
        self.setCursor(Qt.ArrowCursor)
        self.apply_shadows()

        # Top-down composite layout structure
        self.root_layout = QVBoxLayout(self)
        bw = self.layout_policy.border_width
        self.root_layout.setContentsMargins(bw, bw, bw, bw)
        self.root_layout.setSpacing(0)

        # Titlebar element instantiation
        self.titlebar = None
        if self.layout_policy.title_position != TitleBarPosition.NONE:
            self.titlebar = TitleBarElement(self.app, self.layout_policy, self)
            ctrls = self.titlebar.controls
            if ctrls.min_button:
                ctrls.min_button.clicked.connect(lambda: self.requestMinimize.emit(self))
            if ctrls.max_button:
                ctrls.max_button.clicked.connect(lambda: self.requestMaximize.emit(self))
            if ctrls.close_button:
                ctrls.close_button.clicked.connect(lambda: self.requestClose.emit(self))

        # Content container widget
        self.content_container = QWidget(self)
        self.content_container.setObjectName("contentContainer")
        self.content_layout = QVBoxLayout(self.content_container)
        self.content_layout.setContentsMargins(0, 0, 0, 0)
        self.content_layout.setSpacing(0)

        if self.layout_policy.title_position == TitleBarPosition.TOP and self.titlebar:
            self.root_layout.addWidget(self.titlebar, 0)
            self.root_layout.addWidget(self.content_container, 1)
        elif self.layout_policy.title_position == TitleBarPosition.BOTTOM and self.titlebar:
            self.root_layout.addWidget(self.content_container, 1)
            self.root_layout.addWidget(self.titlebar, 0)
        else:
            self.root_layout.addWidget(self.content_container, 1)

        # Initial layout root
        if pane is not None:
            initial_pane = pane
        elif self.app is not None:
            initial_pane = WindowPane(portal, app=self.app)
        else:
            initial_pane = WindowPane(portal, app=None)

        initial_pane.undockRequested.connect(self.undock_pane)
        initial_pane.closeRequested.connect(self.close_pane)
        initial_pane.splitRequested.connect(self.split_pane)

        self.layout_root = initial_pane
        self.content_layout.addWidget(self.layout_root)

        # Edge Split Trigger Buttons (hover handles on outer window borders)
        self.btn_split_left = QPushButton("◧", self)
        self.btn_split_right = QPushButton("◨", self)
        self.btn_split_top = QPushButton("⬒", self)
        self.btn_split_bottom = QPushButton("⬓", self)

        for btn, tip, orient, pos_side in (
            (self.btn_split_left, "Split Left (Full Height)", Qt.Horizontal, "left"),
            (self.btn_split_right, "Split Right (Full Height)", Qt.Horizontal, "right"),
            (self.btn_split_top, "Split Top (Full Width)", Qt.Vertical, "top"),
            (self.btn_split_bottom, "Split Bottom (Full Width)", Qt.Vertical, "bottom"),
        ):
            btn.setObjectName("edgeSplitTrigger")
            btn.setFixedSize(24, 24)
            btn.setToolTip(tip)
            btn.setFocusPolicy(Qt.NoFocus)
            btn.setCursor(Qt.PointingHandCursor)
            btn.clicked.connect(lambda chk=False, o=orient, p=pos_side: self.split(o, p))
            btn.hide()

        # Drop Preview Overlay
        self.drop_overlay = DropPreviewOverlay(self)

        self.show()
        self._apply_geometry()
        self.update_titlebar_and_taskbar()

    @property
    def panes(self):
        return self.get_all_panes()

    def get_all_panes(self, root=None):
        if root is None:
            root = self.layout_root
        if root is None:
            return []
        if isinstance(root, WindowPane):
            return [root]
        if isinstance(root, QSplitter):
            panes = []
            for i in range(root.count()):
                w = root.widget(i)
                panes.extend(self.get_all_panes(w))
            return panes
        return []

    def split(self, orientation, position):
        """Outer window split triggered by outer border buttons."""
        self.split_pane(None, orientation, position)
        self.hide_edge_triggers()

    def _safe_disconnect(self, signal):
        import warnings
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            try:
                signal.disconnect()
            except:
                pass

    def split_pane(self, target_pane, orientation, position, new_pane=None):
        """Splits a specific target_pane (or the root container) horizontally or vertically."""
        if new_pane is None:
            new_pane = WindowPane(self.portal, app=None)

        self._safe_disconnect(new_pane.undockRequested)
        self._safe_disconnect(new_pane.closeRequested)
        self._safe_disconnect(new_pane.splitRequested)

        new_pane.undockRequested.connect(self.undock_pane)
        new_pane.closeRequested.connect(self.close_pane)
        new_pane.splitRequested.connect(self.split_pane)

        # Case 1: Outer split or single pane root
        if target_pane is None or target_pane == self.layout_root or isinstance(self.layout_root, WindowPane):
            if isinstance(self.layout_root, WindowPane):
                old_pane = self.layout_root
                splitter = PortalSplitter(orientation, self.content_container)

                self.content_layout.removeWidget(old_pane)
                if position in ("before", "left", "top"):
                    splitter.addWidget(new_pane)
                    splitter.addWidget(old_pane)
                else:
                    splitter.addWidget(old_pane)
                    splitter.addWidget(new_pane)

                self.content_layout.addWidget(splitter)
                self.layout_root = splitter
                splitter.setSizes([500, 500])
            elif isinstance(self.layout_root, QSplitter):
                if self.layout_root.orientation() == orientation:
                    if position in ("before", "left", "top"):
                        self.layout_root.insertWidget(0, new_pane)
                    else:
                        self.layout_root.addWidget(new_pane)
                    num = self.layout_root.count()
                    self.layout_root.setSizes([1000 // num] * num)
                else:
                    old_root = self.layout_root
                    new_root = PortalSplitter(orientation, self.content_container)

                    self.content_layout.removeWidget(old_root)
                    if position in ("before", "left", "top"):
                        new_root.addWidget(new_pane)
                        new_root.addWidget(old_root)
                    else:
                        new_root.addWidget(old_root)
                        new_root.addWidget(new_pane)

                    self.content_layout.addWidget(new_root)
                    self.layout_root = new_root
                    new_root.setSizes([500, 500])

            self.update_titlebar_and_taskbar()
            orient_name = "Horizontal" if orientation == Qt.Horizontal else "Vertical"
            self.portal.log("EVENT", "SPLIT", f"Split pane ({orient_name}) in '{self.app.display_name if self.app else 'Window'}'")
            return new_pane

        # Case 2: target_pane is inside an existing splitter
        parent = target_pane.parent()
        while parent and not isinstance(parent, QSplitter):
            parent = parent.parent()

        if isinstance(parent, QSplitter):
            idx = parent.indexOf(target_pane)
            if parent.orientation() == orientation:
                if position in ("before", "left", "top"):
                    parent.insertWidget(idx, new_pane)
                else:
                    parent.insertWidget(idx + 1, new_pane)
                num = parent.count()
                parent.setSizes([1000 // num] * num)
            else:
                old_sizes = parent.sizes()
                sub_splitter = PortalSplitter(orientation, parent)

                target_pane.setParent(sub_splitter)
                if position in ("before", "left", "top"):
                    sub_splitter.addWidget(new_pane)
                    sub_splitter.addWidget(target_pane)
                else:
                    sub_splitter.addWidget(target_pane)
                    sub_splitter.addWidget(new_pane)

                parent.insertWidget(idx, sub_splitter)
                sub_splitter.setSizes([500, 500])
                parent.setSizes(old_sizes)

        self.update_titlebar_and_taskbar()
        orient_name = "Horizontal" if orientation == Qt.Horizontal else "Vertical"
        self.portal.log("EVENT", "SPLIT", f"Split sub-pane ({orient_name}) in '{self.app.display_name if self.app else 'Window'}'")
        return new_pane

    def dock_window(self, source_window, target_pane=None, zone="right"):
        """Docks all panes from source_window into self at target_pane or outer window edge."""
        if isinstance(target_pane, str):
            zone = target_pane
            target_pane = None

        panes_to_add = source_window.get_all_panes()
        source_window.layout_root = None

        for p in panes_to_add:
            p.setParent(None)
            self._safe_disconnect(p.undockRequested)
            self._safe_disconnect(p.closeRequested)
            self._safe_disconnect(p.splitRequested)

        orient = Qt.Horizontal if zone in ("left", "right") else Qt.Vertical
        pos = "before" if zone in ("left", "top") else "after"

        curr_target = target_pane
        ordered_panes = list(reversed(panes_to_add)) if pos == "before" else list(panes_to_add)

        for p in ordered_panes:
            curr_target = self.split_pane(curr_target, orient, pos, new_pane=p)

        src_name = source_window.app.display_name if source_window.app else "Window"
        self.portal.log("EVENT", "DOCK", f"Docked '{src_name}' into '{self.app.display_name if self.app else 'Window'}' (zone={zone})")

    def undock_pane(self, pane):
        panes = self.get_all_panes()
        if pane not in panes:
            return

        pane_name = pane.app.display_name if pane.app else "Pane"

        parent = pane.parent()
        while parent and not isinstance(parent, QSplitter) and parent != self.content_container:
            parent = parent.parent()

        if pane == self.layout_root:
            self.content_layout.removeWidget(pane)
            self.layout_root = None

        pane.setParent(None)
        self._safe_disconnect(pane.undockRequested)
        self._safe_disconnect(pane.closeRequested)
        self._safe_disconnect(pane.splitRequested)

        if isinstance(parent, QSplitter):
            self._cleanup_splitter(parent)

        # Spawn new standalone window for this pane
        gx = min(self.portal.desktop.width() - 320, self.x() + 40)
        gy = min(self.portal.desktop.height() - 240, self.y() + 40)
        gw = max(pane.app.layout_policy.min_width if pane.app else 400, 480)
        gh = max(pane.app.layout_policy.min_height if pane.app else 300, 360)
        geom = WindowGeometry(gx, gy, gw, gh)

        new_window = PortalWindow(self.portal, app=pane.app, geometry=geom, pane=pane)
        new_window.requestClose.connect(self.portal.desktop.remove_window)
        new_window.requestMinimize.connect(self.portal.desktop.minimize)
        new_window.requestMaximize.connect(self.portal.desktop.toggle_maximize)

        self.portal.desktop.add_window(new_window)
        self.portal.desktop.focus_window(new_window)

        self.portal.log("EVENT", "UNDOCK", f"Undocked '{pane_name}' from '{self.app.display_name if self.app else 'Window'}'")

        remaining = self.get_all_panes()
        if len(remaining) == 0:
            self.portal.desktop.remove_window(self)
        else:
            self.update_titlebar_and_taskbar()

    def close_pane(self, pane):
        panes = self.get_all_panes()
        if pane not in panes:
            return

        parent = pane.parent()
        while parent and not isinstance(parent, QSplitter) and parent != self.content_container:
            parent = parent.parent()

        if pane == self.layout_root:
            self.content_layout.removeWidget(pane)
            self.layout_root = None

        pane.setParent(None)
        pane.deleteLater()

        if isinstance(parent, QSplitter):
            self._cleanup_splitter(parent)

        remaining = self.get_all_panes()
        if len(remaining) == 0:
            self.portal.desktop.remove_window(self)
        else:
            self.update_titlebar_and_taskbar()

    def _cleanup_splitter(self, splitter):
        if splitter.count() == 0:
            if splitter == self.layout_root:
                self.layout_root = None
            else:
                p = splitter.parent()
                if isinstance(p, QSplitter):
                    splitter.setParent(None)
                    splitter.deleteLater()
                    self._cleanup_splitter(p)
        elif splitter.count() == 1:
            child = splitter.widget(0)
            if splitter == self.layout_root:
                if isinstance(child, WindowPane):
                    self.content_layout.removeWidget(splitter)
                    child.setParent(self.content_container)
                    self.content_layout.addWidget(child)
                    splitter.deleteLater()
                    self.layout_root = child
            else:
                p = splitter.parent()
                if isinstance(p, QSplitter):
                    idx = p.indexOf(splitter)
                    p_sizes = p.sizes()
                    child.setParent(p)
                    p.insertWidget(idx, child)
                    splitter.setParent(None)
                    splitter.deleteLater()
                    p.setSizes(p_sizes)

    def update_titlebar_and_taskbar(self):
        panes = self.get_all_panes()
        if not panes:
            return

        if len(panes) == 1:
            p = panes[0]
            if p.app:
                title = f"{p.app.icon}  {p.app.display_name}"
            else:
                title = "◫  New Split Slot"
            p.header.undock_btn.setVisible(False)
        else:
            titles = [f"{p.app.icon} {p.app.display_name}" if p.app else "◫ Split" for p in panes]
            title = "   |   ".join(titles)
            for p in panes:
                p.header.undock_btn.setVisible(True)

        if self.titlebar:
            self.titlebar.set_title(title)

        if hasattr(self.portal, "taskbar") and hasattr(self.portal, "desktop"):
            self.portal.taskbar.refresh(self.portal.desktop.windows, self.portal.desktop._focused)

    def start_window_drag(self, global_pos):
        if not self.behavior_policy.allow_drag:
            return
        self.portal.desktop.focus_window(self)
        self.dragging = True
        self.drag_offset = global_pos - self.pos()

    def continue_window_drag(self, global_pos):
        if not self.dragging:
            return
        new_pos = global_pos - self.drag_offset
        self.move(new_pos)
        self.geometry_state.x = self.x()
        self.geometry_state.y = self.y()

        desktop_cursor = self.portal.desktop.mapFromGlobal(global_pos)
        drop_target_win = None
        drop_target_pane = None
        drop_zone = None

        for other_win in self.portal.desktop.windows:
            if other_win is not self and other_win.isVisible() and other_win.state != WindowState.MINIMIZED:
                if other_win.geometry().contains(desktop_cursor):
                    drop_target_win = other_win
                    p_local = other_win.mapFromGlobal(global_pos)
                    tw = other_win.width()
                    th = other_win.height()
                    tb_h = other_win.titlebar.height() if other_win.titlebar and other_win.titlebar.isVisible() else 0

                    # Check outer edges first (outer 24px)
                    if p_local.x() <= 24:
                        drop_zone = "left"
                    elif p_local.x() >= tw - 24:
                        drop_zone = "right"
                    elif p_local.y() - tb_h <= 24:
                        drop_zone = "top"
                    elif th - p_local.y() <= 24:
                        drop_zone = "bottom"
                    else:
                        # Find specific pane under cursor
                        panes = other_win.get_all_panes()
                        for p in panes:
                            p_pt = p.mapFromGlobal(global_pos)
                            if p.rect().contains(p_pt):
                                drop_target_pane = p
                                pw = p.width()
                                ph = p.height()
                                dx = p_pt.x() / max(1, pw)
                                dy = p_pt.y() / max(1, ph)

                                d_l = dx
                                d_r = 1.0 - dx
                                d_t = dy
                                d_b = 1.0 - dy
                                min_d = min(d_l, d_r, d_t, d_b)
                                if min_d == d_l:
                                    drop_zone = "left"
                                elif min_d == d_r:
                                    drop_zone = "right"
                                elif min_d == d_t:
                                    drop_zone = "top"
                                else:
                                    drop_zone = "bottom"
                                break
                        if not drop_zone:
                            drop_zone = "right"
                    break

        if drop_target_win:
            if self._last_drop_win and self._last_drop_win != drop_target_win:
                self._last_drop_win.hide_drop_preview()
            drop_target_win.show_drop_preview(drop_zone, target_pane=drop_target_pane)
            self._last_drop_win = drop_target_win
            self._last_drop_pane = drop_target_pane
            self._last_drop_zone = drop_zone
        else:
            if self._last_drop_win:
                self._last_drop_win.hide_drop_preview()
                self._last_drop_win = None
                self._last_drop_pane = None
                self._last_drop_zone = None

    def end_window_drag(self, global_pos):
        self.dragging = False
        target_win = self._last_drop_win
        target_pane = self._last_drop_pane
        zone = self._last_drop_zone

        if target_win:
            target_win.hide_drop_preview()
            self._last_drop_win = None
            self._last_drop_pane = None
            self._last_drop_zone = None

            # Dock into target window
            target_win.dock_window(self, target_pane=target_pane, zone=zone)
            self.portal.desktop.remove_window(self)
            self.portal.desktop.focus_window(target_win)

    def show_drop_preview(self, zone, target_pane=None):
        self.drop_overlay.show_preview(self, zone, target_pane=target_pane)

    def hide_drop_preview(self):
        self.drop_overlay.hide()

    def _position_edge_triggers(self):
        th = self.titlebar.height() if self.titlebar and self.titlebar.isVisible() else 0
        w = self.width()
        h = self.height()
        ch = h - th

        self.btn_split_left.move(4, th + ch // 2 - 12)
        self.btn_split_right.move(w - 28, th + ch // 2 - 12)
        self.btn_split_top.move(w // 2 - 12, th + 4)
        self.btn_split_bottom.move(w // 2 - 12, h - 28)

        for btn in (self.btn_split_left, self.btn_split_right, self.btn_split_top, self.btn_split_bottom):
            btn.raise_()

    def show_edge_triggers(self):
        self._position_edge_triggers()
        self.btn_split_left.show()
        self.btn_split_right.show()
        self.btn_split_top.show()
        self.btn_split_bottom.show()

    def hide_edge_triggers(self):
        self.btn_split_left.hide()
        self.btn_split_right.hide()
        self.btn_split_top.hide()
        self.btn_split_bottom.hide()

    def apply_shadows(self):
        if self.portal.settings.get("window_shadows", False):
            shadow = QGraphicsDropShadowEffect(self)
            shadow.setBlurRadius(20)
            shadow.setColor(QColor(0, 0, 0, 150))
            shadow.setOffset(0, 4)
            self.setGraphicsEffect(shadow)
        else:
            self.setGraphicsEffect(None)

    def _apply_geometry(self):
        r = self.geometry_state
        self.setGeometry(r.x, r.y, r.width, r.height)
        self._position_edge_triggers()

    def set_focus_visual(self, focused):
        self.setProperty("focused", focused)
        self.style().unpolish(self)
        self.style().polish(self)
        if self.titlebar:
            self.titlebar.setProperty("focused", focused)
            self.titlebar.style().unpolish(self.titlebar)
            self.titlebar.style().polish(self.titlebar)

    def mousePressEvent(self, event):
        if event.button() != Qt.LeftButton:
            return super().mousePressEvent(event)

        self.portal.desktop.focus_window(self)

        pos = event.position().toPoint()
        global_pos = event.globalPosition().toPoint()

        if self.behavior_policy.allow_resize:
            edges = self._edges_at(pos)
            if edges:
                self.resizing = True
                self.resize_edges = edges
                self.drag_start_global = global_pos
                self.drag_start_geometry = WindowGeometry(
                    self.geometry_state.x, self.geometry_state.y,
                    self.geometry_state.width, self.geometry_state.height
                )
                event.accept()
                return

        if self.behavior_policy.allow_drag and self.titlebar and self.titlebar.isVisible():
            if self.titlebar.geometry().contains(pos):
                child = self.childAt(pos)
                if not isinstance(child, QPushButton):
                    self.start_window_drag(global_pos)
                    event.accept()
                    return

        super().mousePressEvent(event)

    def mouseMoveEvent(self, event):
        pos = event.position().toPoint()
        global_pos = event.globalPosition().toPoint()

        if self.dragging and self.behavior_policy.allow_drag:
            self.continue_window_drag(global_pos)
            event.accept()
            return

        if self.resizing and self.behavior_policy.allow_resize:
            self._resize_from(global_pos)
            event.accept()
            return

        if self.behavior_policy.allow_resize:
            edges = self._edges_at(pos)
            self.setCursor(self._cursor_for_edges(edges))
        else:
            self.setCursor(Qt.ArrowCursor)

        # Edge hover detection to show/hide split triggers
        th = self.titlebar.height() if self.titlebar and self.titlebar.isVisible() else 0
        w = self.width()
        h = self.height()
        near_edge = (
            pos.x() <= 28 or
            pos.x() >= w - 28 or
            (pos.y() >= th and pos.y() <= th + 28) or
            pos.y() >= h - 28
        )
        if near_edge:
            self.show_edge_triggers()
        else:
            self.hide_edge_triggers()

        super().mouseMoveEvent(event)

    def mouseReleaseEvent(self, event):
        if self.dragging:
            self.end_window_drag(event.globalPosition().toPoint())

        self.dragging = False
        self.resizing = False
        self.resize_edges.clear()
        self.unsetCursor()
        super().mouseReleaseEvent(event)

    def leaveEvent(self, event):
        self.hide_edge_triggers()
        super().leaveEvent(event)

    def mouseDoubleClickEvent(self, event):
        if (
            event.button() == Qt.LeftButton and
            self.behavior_policy.allow_maximize and
            self.titlebar and
            self.titlebar.geometry().contains(event.position().toPoint())
        ):
            self.requestMaximize.emit(self)
            event.accept()
            return
        super().mouseDoubleClickEvent(event)

    def resizeEvent(self, event):
        self._apply_geometry()
        super().resizeEvent(event)

    def _edges_at(self, p):
        m = self.layout_policy.resize_margin
        edges = set()
        if p.x() <= m:
            edges.add("W")
        elif p.x() >= self.width() - m:
            edges.add("E")
        if p.y() <= m:
            edges.add("N")
        elif p.y() >= self.height() - m:
            edges.add("S")
        return edges

    @staticmethod
    def _cursor_for_edges(edges):
        if edges == {"N", "W"} or edges == {"S", "E"}:
            return Qt.SizeFDiagCursor
        if edges == {"N", "E"} or edges == {"S", "W"}:
            return Qt.SizeBDiagCursor
        if edges & {"N", "S"}:
            return Qt.SizeVerCursor
        if edges & {"E", "W"}:
            return Qt.SizeHorCursor
        return Qt.ArrowCursor

    def _resize_from(self, global_pos):
        if not self.behavior_policy.allow_resize:
            return
        dx = global_pos.x() - self.drag_start_global.x()
        dy = global_pos.y() - self.drag_start_global.y()
        r = self.geometry_state
        start_r = self.drag_start_geometry
        min_w = self.layout_policy.min_width
        min_h = self.layout_policy.min_height

        if "W" in self.resize_edges:
            new_w = max(min_w, start_r.width - dx)
            r.x = start_r.x + (start_r.width - new_w)
            r.width = new_w
        if "E" in self.resize_edges:
            r.width = max(min_w, start_r.width + dx)
        if "N" in self.resize_edges:
            new_h = max(min_h, start_r.height - dy)
            r.y = start_r.y + (start_r.height - new_h)
            r.height = new_h
        if "S" in self.resize_edges:
            r.height = max(min_h, start_r.height + dy)

        self._apply_geometry()


# ---------------------------------------------------------------------------
# Desktop / window manager
# ---------------------------------------------------------------------------

class PortalDesktop(QWidget):
    def __init__(self, portal):
        super().__init__(portal)
        self.portal = portal
        self.windows = []
        self._focused = None
        self.setMouseTracking(True)

        self.explorer = ExplorerView(portal, self, is_desktop=True)
        self.explorer.itemActionTriggered.connect(self.handle_action)

    def handle_action(self, data):
        if data.get("type") == "app":
            self.portal.launch(data["id"])

    def resizeEvent(self, event):
        self.explorer.setGeometry(self.rect())
        super().resizeEvent(event)

    def paintEvent(self, event):
        painter = QPainter(self)
        painter.fillRect(self.rect(), QColor(self.portal.theme.desktop))

        pen = QPen(QColor(self.portal.theme.accent))
        pen.setWidth(1)
        pen.setStyle(Qt.DotLine)
        painter.setPen(pen)
        for x in range(40, self.width(), 80):
            painter.drawLine(x, 0, x, self.height())
        painter.setPen(QColor(self.portal.theme.secondary_text))
        painter.drawText(24, 34, "PORTAL")
        painter.drawText(24, 54, "Qt interface simulator")

    def add_window(self, window):
        self.windows.append(window)
        window.setParent(self)
        window.show()
        self.focus_window(window)
        win_name = window.app.display_name if window.app else "Window"
        self.portal.log("EVENT", "ADD_WINDOW", f"Added '{win_name}' to desktop")

    def remove_window(self, window):
        win_name = window.app.display_name if window.app else "Window"
        if window in self.windows:
            self.windows.remove(window)
        if self._focused is window:
            self._focused = None
        window.deleteLater()
        self.portal.log("EVENT", "CLOSE", f"Closed '{win_name}'")
        if self.windows:
            self.focus_window(self.windows[-1])
        else:
            self.portal.taskbar.refresh(self.windows, None)

    def focus_window(self, window):
        if window not in self.windows:
            return
        for w in self.windows:
            w.set_focus_visual(w is window)
        self._focused = window
        window.raise_()
        self.portal.taskbar.refresh(self.windows, self._focused)
        win_name = window.app.display_name if window.app else "Window"
        self.portal.log("INFO", "FOCUS", f"Focused '{win_name}'")

    def toggle_maximize(self, window):
        if window.state == WindowState.MAXIMIZED:
            if window.geometry_state.previous:
                p = window.geometry_state.previous
                window.geometry_state = WindowGeometry(
                    p.x(), p.y(), p.width(), p.height()
                )
            window.state = WindowState.NORMAL
        else:
            window.geometry_state.previous = QRect(
                window.x(), window.y(), window.width(), window.height()
            )
            min_w = window.layout_policy.min_width
            min_h = window.layout_policy.min_height
            window.geometry_state = WindowGeometry(
                8, 8,
                max(min_w, self.width() - 16),
                max(min_h, self.height() - self.portal.taskbar.height() - 16),
                window.geometry_state.previous
            )
            window.state = WindowState.MAXIMIZED
        window._apply_geometry()
        self.focus_window(window)
        win_name = window.app.display_name if window.app else "Window"
        self.portal.log("EVENT", "MAXIMIZE", f"{'Maximized' if window.state == WindowState.MAXIMIZED else 'Restored'} '{win_name}'")

    def minimize(self, window):
        window.state = WindowState.MINIMIZED
        window.hide()
        if self._focused is window:
            self._focused = None
        self.portal.taskbar.refresh(self.windows, self._focused)
        win_name = window.app.display_name if window.app else "Window"
        self.portal.log("EVENT", "MINIMIZE", f"Minimized '{win_name}'")

    def restore(self, window):
        window.state = WindowState.NORMAL
        window.show()
        self.focus_window(window)
        win_name = window.app.display_name if window.app else "Window"
        self.portal.log("EVENT", "RESTORE", f"Restored '{win_name}'")

    def mousePressEvent(self, event):
        if event.button() == Qt.LeftButton:
            self.clear_focus()
        super().mousePressEvent(event)

    def clear_focus(self):
        for w in self.windows:
            w.set_focus_visual(False)
        self._focused = None
        self.portal.taskbar.refresh(self.windows, None)


# ---------------------------------------------------------------------------
# Taskbar
# ---------------------------------------------------------------------------

class PortalTaskbar(QFrame):
    def __init__(self, portal):
        super().__init__(portal)
        self.portal = portal
        self.setObjectName("taskbar")
        self.setFixedHeight(48)
        self.setMouseTracking(True)

        self.opacity_effect = QGraphicsOpacityEffect(self)
        self.opacity_effect.setOpacity(1.0)
        self.setGraphicsEffect(self.opacity_effect)

        self.opacity_anim = QPropertyAnimation(self.opacity_effect, b"opacity")
        self.opacity_anim.setDuration(200)

        layout = QHBoxLayout(self)
        layout.setContentsMargins(8, 6, 8, 6)
        layout.setSpacing(6)

        self.start = QPushButton("Start")
        self.start.setObjectName("startButton")
        self.start.setSizePolicy(QSizePolicy.Fixed, QSizePolicy.Fixed)
        self.start.clicked.connect(self.show_start)
        layout.addWidget(self.start)

        self.running = QHBoxLayout()
        self.running.setSpacing(4)
        layout.addLayout(self.running, 1)

        self.clock = QLabel()
        self.clock.setAlignment(Qt.AlignCenter)
        self.clock.setMinimumWidth(72)
        layout.addWidget(self.clock)

        timer = QTimer(self)
        timer.timeout.connect(self.update_clock)
        timer.start(1000)
        self.update_clock()

    def enterEvent(self, event):
        if self.portal.settings.get("auto_hide_taskbar", False):
            self.opacity_anim.stop()
            self.opacity_anim.setEndValue(1.0)
            self.opacity_anim.start()
        super().enterEvent(event)

    def leaveEvent(self, event):
        if self.portal.settings.get("auto_hide_taskbar", False):
            self.opacity_anim.stop()
            self.opacity_anim.setEndValue(0.2)
            self.opacity_anim.start()
        super().leaveEvent(event)

    def update_opacity(self):
        if self.portal.settings.get("auto_hide_taskbar", False):
            self.opacity_effect.setOpacity(0.2)
        else:
            self.opacity_effect.setOpacity(1.0)

    def update_clock(self):
        fmt = "HH:mm" if self.portal.settings.get("clock_24hr", True) else "h:mm AP"
        if self.portal.settings.get("show_date", False):
            fmt = "MM/dd/yyyy\n" + fmt
        self.clock.setText(QDateTime.currentDateTime().toString(fmt))

    def refresh(self, windows, focused):
        while self.running.count():
            item = self.running.takeAt(0)
            if item.widget():
                item.widget().deleteLater()

        for window in windows:
            panes = window.get_all_panes() if hasattr(window, "get_all_panes") else []
            if panes:
                if len(panes) == 1:
                    p = panes[0]
                    txt = f"{p.app.icon} {p.app.display_name}" if p.app else "◫ Split"
                else:
                    titles = [f"{p.app.icon} {p.app.display_name}" if p.app else "◫ Split" for p in panes]
                    txt = " | ".join(titles)
            elif window.app:
                txt = f"{window.app.icon} {window.app.display_name}"
            else:
                txt = "Window"

            b = QPushButton(txt)
            b.setCheckable(True)
            b.setChecked(window is focused and window.state != WindowState.MINIMIZED)
            b.clicked.connect(
                lambda checked=False, w=window: self.task_clicked(w)
            )
            self.running.addWidget(b)

    def task_clicked(self, window):
        if window.state == WindowState.MINIMIZED:
            self.portal.desktop.restore(window)
        elif self.portal.desktop._focused is window:
            self.portal.desktop.minimize(window)
        else:
            self.portal.desktop.focus_window(window)

    def show_start(self):
        menu = QMenu(self)
        menu.setStyleSheet(f"""
            QMenu {{
                background: {self.portal.theme.window};
                border: 1px solid {self.portal.theme.window_border};
                color: {self.portal.theme.text};
            }}
            QMenu::item {{
                padding: 6px 24px 6px 24px;
            }}
            QMenu::item:selected {{
                background: {self.portal.theme.button_hover};
            }}
        """)
        for app_id, app in self.portal.apps.items():
            action = menu.addAction(f"{app.icon}  {app.display_name}")
            action.triggered.connect(lambda checked=False, aid=app_id: self.portal.launch(aid))

        menu.addSeparator()
        menu.addAction("Exit").triggered.connect(QApplication.instance().quit)

        pos = self.start.mapToGlobal(QPoint(0, 0))
        pos.setY(pos.y() - menu.sizeHint().height())
        menu.exec(pos)


# ---------------------------------------------------------------------------
# Main portal
# ---------------------------------------------------------------------------

class Portal(QMainWindow):
    # Class-level signal declaration (PySide6 requires this)
    eventLogged = Signal(dict)

    def __init__(self):
        super().__init__()
        self.setWindowTitle("BSPT")
        self.setWindowFlags(Qt.FramelessWindowHint | Qt.Window)
        self.resize(1920, 1080)

        self.theme = THEMES["Classic"]

        self.settings = {
            "show_desktop_icons": True,
            "window_shadows": False,
            "auto_hide_taskbar": False,
            "font_size": 10,
            "clock_24hr": True,
            "show_date": False,
            "hidden_columns": []
        }

        # Initialize Business Autonomy System state engine
        self.business_state = BusinessStateEngine(self)

        self.apps = {
            BusinessSupervisorApplication.application_id: BusinessSupervisorApplication(),
            BusinessDatabaseApplication.application_id: BusinessDatabaseApplication(),
            ModelRouterApplication.application_id: ModelRouterApplication(),
            TaskViewerApplication.application_id: TaskViewerApplication(),
            ConsoleApplication.application_id: ConsoleApplication(),
            FileManagerApplication.application_id: FileManagerApplication(),
            GridApplication.application_id: GridApplication(),
            SettingsApplication.application_id: SettingsApplication(),
            AboutApplication.application_id: AboutApplication(),
            HomeApplication.application_id: HomeApplication(),
        }

        # Event logging infrastructure
        self.log_history = []

        self.desktop = PortalDesktop(self)
        self.taskbar = PortalTaskbar(self)

        root = QWidget()
        root.setObjectName("root")
        layout = QVBoxLayout(root)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(0)
        layout.addWidget(self.desktop, 1)
        layout.addWidget(self.taskbar, 0)
        self.setCentralWidget(root)

        self.populate_desktop()
        self.apply_theme()

    def log(self, level, category, message):
        """Central event logger — emits eventLogged signal for live console/viewer updates."""
        entry = {
            "timestamp": datetime.datetime.now().strftime('%H:%M:%S'),
            "level": level,
            "category": category,
            "message": message,
        }
        self.log_history.append(entry)
        self.eventLogged.emit(entry)

    def populate_desktop(self):
        for app_id in ("autonomy", "bizdb", "agents", "tasks", "console", "files", "grid", "settings", "about", "home"):
            app = self.apps.get(app_id)
            if not app:
                continue
            self.desktop.explorer.add_item(app.display_name, app.icon, {"type": "app", "id": app_id})

    def launch(self, application_id, x=None, y=None):
        app = self.apps[application_id]
        w, h = app.default_size
        if x is None:
            x = 120 + len(self.desktop.windows) * 28
        if y is None:
            y = 80 + len(self.desktop.windows) * 24

        g = WindowGeometry(
            max(8, x), max(8, y),
            min(w, max(app.layout_policy.min_width, self.desktop.width() - 16)),
            min(h, max(app.layout_policy.min_height, self.desktop.height() - 16))
        )
        window = PortalWindow(self, app=app, geometry=g, layout_policy=app.layout_policy, behavior_policy=app.behavior_policy)
        window.requestClose.connect(self.desktop.remove_window)
        window.requestMinimize.connect(self.desktop.minimize)
        window.requestMaximize.connect(self.desktop.toggle_maximize)
        self.desktop.add_window(window)
        self.desktop.focus_window(window)
        # Log launch event
        self.log("EVENT", "LAUNCH", f"Launched {application_id}")
        return window

    def set_theme(self, name):
        if name not in THEMES:
            return
        self.theme = THEMES[name]
        self.apply_theme()
        self.log("EVENT", "THEME_CHANGE", f"Theme changed to '{name}'")
        self.status(f"Theme: {name}")

    def update_setting(self, key, value):
        self.settings[key] = value
        self.log("EVENT", "SETTING_UPDATE", f"Setting '{key}' changed to {value}")
        if key == "show_desktop_icons":
            self.desktop.explorer.setVisible(value)
        elif key == "window_shadows":
            for w in self.desktop.windows:
                w.apply_shadows()
        elif key == "auto_hide_taskbar":
            self.taskbar.update_opacity()
        elif key in ("clock_24hr", "show_date"):
            self.taskbar.update_clock()
        elif key == "font_size":
            self.apply_theme()

    def apply_theme(self):
        t = self.theme
        fs = self.settings.get("font_size", 10)
        self.setStyleSheet(f"""
        QWidget {{
            font-family: "Segoe UI", "Noto Sans", sans-serif;
            font-size: {fs}pt;
            color: {t.text};
        }}
        QWidget#root {{
            background: {t.desktop};
        }}
        QFrame#portalWindow {{
            background: {t.window};
            border: 1px solid {t.window_border};
        }}
        QListWidget#desktopExplorer {{
            background: transparent;
            border: none;
            color: {t.desktop_text};
            font-size: 11pt;
        }}
        QListWidget#desktopExplorer::item:hover {{
            background: rgba(255, 255, 255, 0.1);
            border-radius: 4px;
        }}
        QListWidget#desktopExplorer::item:selected {{
            background: rgba(255, 255, 255, 0.2);
            border-radius: 4px;
        }}
        QListWidget#windowExplorer {{
            background: {t.panel};
            border: 1px solid {t.window_border};
            color: {t.text};
        }}
        QListWidget#windowExplorer::item:hover {{
            background: {t.button_hover};
            border-radius: 4px;
        }}
        QListWidget#windowExplorer::item:selected {{
            background: {t.button_pressed};
            border-radius: 4px;
        }}
        QPushButton {{
            background: {t.button};
            color: {t.text};
            border: 1px solid {t.window_border};
            border-radius: 3px;
            padding: 5px 10px;
        }}
        QPushButton:hover {{
            background: {t.button_hover};
        }}
        QPushButton:pressed {{
            background: {t.button_pressed};
        }}
        QPushButton#startButton {{
            font-weight: bold;
            min-width: 72px;
        }}
        QFrame#taskbar {{
            background: {t.taskbar};
            border-top: 1px solid {t.taskbar_border};
        }}
        QFrame#taskbar QLabel {{
            color: {t.text};
        }}
        QFrame#titlebar {{
            background: {t.title_inactive};
        }}
        QFrame#titlebar[focused="true"] {{
            background: {t.title_active};
        }}
        QLabel#windowTitle {{
            color: {t.title_text};
            font-weight: 600;
        }}
        QWidget#contentContainer {{
            background: {t.window};
        }}
        QLabel#sectionTitle {{
            font-size: 16pt;
            font-weight: 600;
            color: {t.text};
        }}
        QLabel#pathLabel {{
            color: {t.secondary_text};
        }}
        QListWidget, QComboBox {{
            background: {t.panel};
            color: {t.text};
            border: 1px solid {t.window_border};
            padding: 4px;
        }}
        QSlider::groove:horizontal {{
            height: 4px;
            background: {t.window_border};
        }}
        QSlider::handle:horizontal {{
            width: 14px;
            margin: -5px 0;
            background: {t.accent};
            border-radius: 7px;
        }}
        QCheckBox {{
            spacing: 8px;
        }}

        /* Splitter styling */
        QSplitter#windowSplitter {{
            background: transparent;
        }}
        QSplitter::handle:horizontal {{
            background: {t.window_border};
            width: 4px;
        }}
        QSplitter::handle:vertical {{
            background: {t.window_border};
            height: 4px;
        }}
        QSplitter::handle:hover {{
            background: {t.accent};
        }}

        /* Window Pane styling */
        QFrame#windowPane {{
            background: {t.window};
            border: 1px solid {t.window_border};
        }}
        QFrame#paneHeader {{
            background: {t.button};
            border-bottom: 1px solid {t.window_border};
        }}
        QLabel#paneTitleLabel {{
            font-size: 9pt;
            font-weight: 600;
            color: {t.text};
        }}
        QPushButton#paneUndockButton, QPushButton#paneSplitHButton, QPushButton#paneSplitVButton {{
            background: transparent;
            border: none;
            font-size: 10pt;
            color: {t.secondary_text};
            padding: 0px;
            border-radius: 2px;
        }}
        QPushButton#paneUndockButton:hover, QPushButton#paneSplitHButton:hover, QPushButton#paneSplitVButton:hover {{
            background: {t.button_hover};
            color: {t.text};
        }}
        QPushButton#paneCloseButton {{
            background: transparent;
            border: none;
            font-size: 11pt;
            font-weight: bold;
            color: {t.secondary_text};
            padding: 0px;
            border-radius: 2px;
        }}
        QPushButton#paneCloseButton:hover {{
            background: #e81123;
            color: #ffffff;
        }}

        /* Placeholder styling */
        QFrame#panePlaceholder {{
            background: {t.panel};
            border: 2px dashed {t.window_border};
            border-radius: 6px;
            margin: 8px;
            padding: 12px;
        }}
        QLabel#placeholderIcon {{
            font-size: 26pt;
            color: {t.accent};
        }}
        QLabel#placeholderText {{
            font-size: 9pt;
            font-weight: 500;
            color: {t.secondary_text};
        }}
        QPushButton#placeholderAppBtn {{
            background: {t.button};
            color: {t.text};
            border: 1px solid {t.window_border};
            border-radius: 4px;
            padding: 5px 10px;
            font-size: 9pt;
        }}
        QPushButton#placeholderAppBtn:hover {{
            background: {t.button_hover};
            border-color: {t.accent};
        }}

        /* Edge split trigger buttons */
        QPushButton#edgeSplitTrigger {{
            background: {t.accent};
            color: #ffffff;
            border: 1px solid {t.window_border};
            border-radius: 12px;
            font-size: 9pt;
            font-weight: bold;
            padding: 0px;
        }}
        QPushButton#edgeSplitTrigger:hover {{
            background: {t.button_hover};
            color: {t.text};
        }}

        /* Drop Preview Overlay */
        QFrame#dropPreviewOverlay {{
            background: rgba(76, 142, 216, 0.25);
            border: 2px dashed {t.accent};
            border-radius: 4px;
        }}
        QLabel#dropPreviewLabel {{
            background: {t.accent};
            color: #ffffff;
            padding: 6px 12px;
            border-radius: 4px;
            font-weight: bold;
            font-size: 10pt;
        }}

        /* Task Viewer styling */
        QFrame#taskStatsFrame {{
            background: {t.panel};
            border: 1px solid {t.window_border};
            border-radius: 4px;
        }}
        QLabel#taskMetricBadge {{
            font-weight: 600;
            color: {t.accent};
            padding: 2px 8px;
        }}
        QLineEdit#taskSearchInput {{
            background: {t.panel};
            color: {t.text};
            border: 1px solid {t.window_border};
            border-radius: 3px;
            padding: 4px 8px;
        }}
        QTreeWidget#taskTree {{
            background: {t.panel};
            color: {t.text};
            border: 1px solid {t.window_border};
            alternate-background-color: {t.button};
        }}
        QTreeWidget#taskTree::item:selected {{
            background: {t.accent};
            color: #ffffff;
        }}
        QTreeWidget#taskTree::item:hover {{
            background: {t.button_hover};
        }}
        QHeaderView::section {{
            background: {t.button};
            color: {t.text};
            border: 1px solid {t.window_border};
            padding: 4px;
            font-weight: 600;
        }}

        /* Console styling */
        QTextEdit#consoleOutputArea {{
            background: #1a1a2e;
            color: #e0e0e0;
            border: 1px solid {t.window_border};
            border-radius: 3px;
            padding: 6px;
            font-family: "Consolas", "Courier New", monospace;
        }}
        QLabel#consolePromptLabel {{
            font-family: "Consolas", "Courier New", monospace;
            font-weight: bold;
            color: {t.accent};
            padding: 0 4px;
        }}
        QLineEdit#consoleCmdInput {{
            background: {t.panel};
            color: {t.text};
            border: 1px solid {t.window_border};
            border-radius: 3px;
            padding: 4px 8px;
            font-family: "Consolas", "Courier New", monospace;
        }}
        QPushButton#consoleRunButton {{
            background: {t.accent};
            color: #ffffff;
            border: 1px solid {t.window_border};
            border-radius: 3px;
            padding: 4px 12px;
            font-weight: bold;
        }}
        QPushButton#consoleRunButton:hover {{
            background: {t.button_hover};
            color: {t.text};
        }}

        /* TabWidget styling */
        QTabWidget::pane {{
            border: 1px solid {t.window_border};
            background: {t.panel};
            border-radius: 4px;
        }}
        QTabBar::tab {{
            background: {t.button};
            color: {t.text};
            padding: 6px 12px;
            margin-right: 2px;
            border: 1px solid {t.window_border};
            border-bottom: none;
            border-top-left-radius: 4px;
            border-top-right-radius: 4px;
        }}
        QTabBar::tab:selected {{
            background: {t.panel};
            color: {t.accent};
            font-weight: bold;
        }}
        QTabBar::tab:hover:!selected {{
            background: {t.button_hover};
        }}

        /* Business Autonomy Operator UI styling */
        QFrame#operatorModeFrame {{
            background: {t.panel};
            border: 1px solid {t.window_border};
            border-radius: 6px;
        }}
        QPushButton#modeBtnGreen {{
            background: #1b4332;
            color: #74c69d;
            border: 1px solid #2d6a4f;
            border-radius: 3px;
            font-weight: bold;
            padding: 4px 10px;
        }}
        QPushButton#modeBtnGreen:hover {{
            background: #2d6a4f;
            color: #ffffff;
        }}
        QPushButton#modeBtnYellow {{
            background: #4a3800;
            color: #ffd60a;
            border: 1px solid #735700;
            border-radius: 3px;
            font-weight: bold;
            padding: 4px 10px;
        }}
        QPushButton#modeBtnYellow:hover {{
            background: #735700;
            color: #ffffff;
        }}
        QPushButton#modeBtnRed {{
            background: #491114;
            color: #ff858d;
            border: 1px solid #781d22;
            border-radius: 3px;
            font-weight: bold;
            padding: 4px 10px;
        }}
        QPushButton#modeBtnRed:hover {{
            background: #781d22;
            color: #ffffff;
        }}
        QFrame#delegationCard {{
            background: {t.window};
            border: 1px solid {t.window_border};
            border-radius: 6px;
        }}
        QTableWidget {{
            background: {t.panel};
            color: {t.text};
            border: 1px solid {t.window_border};
            gridline-color: {t.button};
        }}
        QTableWidget::item:selected {{
            background: {t.accent};
            color: #ffffff;
        }}
        QTableWidget::item:hover {{
            background: {t.button_hover};
        }}
        """)

        for w in getattr(self.desktop, "windows", []):
            w.set_focus_visual(w is self.desktop._focused)
        self.desktop.update()
        self.taskbar.update()

    def status(self, message):
        # Keep the simulator intentionally self-contained.
        print(message)


if __name__ == "__main__":
    app = QApplication(sys.argv)
    app.setApplicationName("BSPT")
    window = Portal()
    window.show()
    sys.exit(app.exec())

