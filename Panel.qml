import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui
import "Model.js" as Model

Panel {
  id: root
  moduleName: "io.github.alinuxfan.pixelbudspro"
  ipcTarget: "pixelbudspro"
  manageIpc: false

  property int cursorIndex: 0
  property bool cursorActive: false

  readonly property bool hideWhenDisconnected: setting("hideWhenDisconnected", true) === true
  readonly property color foreground: bar ? bar.foreground : Color.foreground
  readonly property color urgent: bar ? bar.urgent : Color.urgent
  readonly property color dim: Qt.darker(foreground, 1.55)
  readonly property color barIconColor: buds.hasBuds ? barForeground : Qt.darker(barForeground, 1.55)
  readonly property string fontFamily: bar ? bar.fontFamily : Style.font.family
  // Say nothing extra once a section already explains the state.
  readonly property bool guidanceVisible: !buds.hasBuds && !buds.hasBattery && !buds.schemaUnsupported

  property int phraseIndex: 0

  readonly property int lowBatteryPercent: 20
  readonly property int phraseIntervalMs: 2800

  // A short rotation, same spirit as omapods' ten: something to look at while the
  // panel sits open, never load-bearing information.
  readonly property var activePhrases: [
    "Fast Paired",
    "Casting to nowhere in particular",
    "Assistant is listening (probably)",
    "Silent Seal engaged",
    "Handoff pending",
    "Tensor is thinking"
  ]
  readonly property string heroPhraseText: activePhrases[phraseIndex % activePhrases.length]

  readonly property var modes: Model.availableModes()

  // Rebuilt whenever a section appears, so j and k never land on a hidden control.
  readonly property var cursorRows: {
    var rows = []
    if (!buds.hasBuds) return rows
    for (var i = 0; i < modes.length; i++) rows.push("mode:" + modes[i])
    rows.push("multipoint")
    rows.push("ohd")
    rows.push("speech")
    rows.push("volumeexposure")
    return rows
  }

  readonly property string cursorRow: cursorRows.length === 0
    ? ""
    : cursorRows[Math.max(0, Math.min(cursorIndex, cursorRows.length - 1))]

  function rowHasCursor(name) {
    return cursorActive && cursorRow === name
  }

  function moveCursor(dy) {
    cursorActive = true
    if (cursorRows.length === 0) return
    cursorIndex = Math.max(0, Math.min(cursorRows.length - 1, cursorIndex + dy))
  }

  function activateCursor() {
    var name = cursorRow
    if (name.indexOf("mode:") === 0) buds.setAncMode(parseInt(name.substring(5), 10))
    else if (name === "multipoint") buds.setMultipointEnabled(!buds.multipointEnabled)
    else if (name === "ohd") buds.setOnHeadDetectionEnabled(!buds.onHeadDetectionEnabled)
    else if (name === "speech") buds.setSpeechDetectionEnabled(!buds.speechDetectionEnabled)
    else if (name === "volumeexposure") buds.setVolumeExposureNotificationsEnabled(!buds.volumeExposureNotificationsEnabled)
  }

  function focusRow(name) {
    var at = cursorRows.indexOf(name)
    if (at < 0) return
    cursorActive = true
    cursorIndex = at
  }

  visible: !hideWhenDisconnected || buds.hasBuds || buds.hasBattery
  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  onOpenedChanged: if (opened) {
    cursorActive = false
    cursorIndex = 0
    if (panelFlick) panelFlick.contentY = 0
    buds.refresh()
    Qt.callLater(function () { keyCatcher.forceActiveFocus() })
  }

  Service {
    id: buds
    settings: root.settings
  }

  IpcHandler {
    target: root.ipcTarget
    function open(): void { root.open() }
    function close(): void { root.close() }
    function toggle(): void { root.toggle() }
    function refresh(): string { buds.refresh(); return "ok" }
    function anc(): string { buds.cycleAncMode(); return "ok" }
    function status(): string { return Model.ancModeName(buds.ancMode) }
  }

  BarIconButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    iconComponent: Component {
      Item {
        PixelBudsIcon {
          anchors.centerIn: parent
          iconSize: Style.space(12)
          color: root.barIconColor
        }
      }
    }
    onPressed: function (buttonCode) {
      if (buttonCode === Qt.RightButton) buds.cycleAncMode()
      else root.toggle()
    }
  }

  KeyboardPanel {
    id: panel
    anchorItem: button
    owner: root
    bar: root.bar
    open: root.opened
    focusTarget: keyCatcher
    contentWidth: panel.fittedContentWidth(Style.space(380))
    contentHeight: panel.fittedContentHeight(column.implicitHeight, Style.space(520))

    PanelKeyCatcher {
      id: keyCatcher
      anchors.fill: parent
      onMoveRequested: function (dx, dy) {
        if (!root.cursorActive) { root.cursorActive = true; return }
        if (dy !== 0) root.moveCursor(dy)
      }
      onActivateRequested: if (root.cursorActive) root.activateCursor()
      onCloseRequested: root.close()
      onTabRequested: function (direction) { root.switchPanel(direction) }
      onTextKey: function (t) {
        var key = String(t).toLowerCase()
        if (key === "r") buds.refresh()
        else if (!buds.hasBuds) return
        else if (key === "o") buds.setAncMode(Model.ANC_OFF)
        else if (key === "n") buds.setAncMode(Model.ANC_ACTIVE)
        else if (key === "t") buds.setAncMode(Model.ANC_AWARE)
        else if (key === "a") buds.setAncMode(Model.ANC_ADAPTIVE)
        else if (key === "m") buds.setMultipointEnabled(!buds.multipointEnabled)
        else if (key === "h") buds.setOnHeadDetectionEnabled(!buds.onHeadDetectionEnabled)
        else if (key === "s") buds.setSpeechDetectionEnabled(!buds.speechDetectionEnabled)
        else if (key === "v") buds.setVolumeExposureNotificationsEnabled(!buds.volumeExposureNotificationsEnabled)
      }

      Flickable {
        id: panelFlick
        anchors.fill: parent
        contentWidth: width
        contentHeight: column.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.VerticalFlick
        interactive: contentHeight > height
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        Column {
          id: column
          width: panelFlick.width
          spacing: Style.space(12)

          PanelHero {
            id: hero
            width: parent.width
            title: buds.modelName !== "" ? buds.modelName : (buds.deviceName !== "" ? buds.deviceName : "Pixel Buds Pro")
            meta: buds.hasBuds ? root.heroPhraseText
              : buds.schemaUnsupported ? "Unsupported status schema"
              : buds.daemonReachable ? "Not connected"
              : "pixelbudsd is not running"
            foreground: root.foreground
            fontFamily: root.fontFamily
            iconOpacity: buds.hasBuds ? 1.0 : 0.5
            iconComponent: Component {
              PixelBudsIcon {
                iconSize: Style.font.display
                color: buds.hasBuds ? root.foreground : root.dim
              }
            }
          }

          Text {
            // A command failure gets its own field, or the next status read wipes it unread.
            visible: buds.actionStatus !== "" || (buds.lastError !== "" && buds.daemonReachable)
            width: parent.width
            text: buds.actionStatus !== "" ? buds.actionStatus : buds.lastError
            color: root.urgent
            font.family: root.fontFamily
            font.pixelSize: Style.font.bodySmall
            wrapMode: Text.WordWrap
          }

          Column {
            visible: buds.hasBattery
            width: parent.width
            spacing: Style.space(10)

            PanelSectionHeader {
              text: "BATTERY"
              foreground: root.foreground
              fontFamily: root.fontFamily
            }

            Column {
              width: parent.width
              spacing: Style.space(6)

              BudRow { width: parent.width; label: "Left"; bud: buds.leftBud }
              BudRow { width: parent.width; label: "Right"; bud: buds.rightBud }
              BudRow {
                width: parent.width
                label: "Case"
                bud: ({ level: buds.caseBattery.level, charging: buds.caseBattery.charging, inCase: false })
              }
            }
          }

          PanelSeparator {
            visible: buds.hasBattery && buds.hasBuds
            foreground: root.foreground
          }

          Column {
            visible: buds.hasBuds
            width: parent.width
            spacing: Style.space(10)

            PanelSectionHeader {
              text: "ANC MODE"
              foreground: root.foreground
              fontFamily: root.fontFamily
            }

            Column {
              width: parent.width
              spacing: Style.space(6)

              Repeater {
                model: root.modes
                ModeRow {
                  required property var modelData
                  width: parent.width
                  mode: modelData
                }
              }
            }
          }

          PanelSeparator {
            visible: buds.hasBuds
            foreground: root.foreground
          }

          Column {
            visible: buds.hasBuds
            width: parent.width
            spacing: Style.space(6)

            ToggleRow {
              width: parent.width
              rowName: "multipoint"
              label: "Multipoint"
              caption: "Stay connected to two devices at once"
              checked: buds.multipointEnabled
              onToggled: buds.setMultipointEnabled(!buds.multipointEnabled)
            }

            ToggleRow {
              width: parent.width
              rowName: "ohd"
              label: "On-head detection"
              caption: "Device-side behavior; doesn't pause playback here"
              checked: buds.onHeadDetectionEnabled
              onToggled: buds.setOnHeadDetectionEnabled(!buds.onHeadDetectionEnabled)
            }

            ToggleRow {
              width: parent.width
              rowName: "speech"
              label: "Speech Detection"
              caption: "Switch to Transparency when you start talking"
              checked: buds.speechDetectionEnabled
              onToggled: buds.setSpeechDetectionEnabled(!buds.speechDetectionEnabled)
            }

            ToggleRow {
              width: parent.width
              rowName: "volumeexposure"
              label: "Volume Notifications"
              caption: "Warn when listening volume gets loud"
              checked: buds.volumeExposureNotificationsEnabled
              onToggled: buds.setVolumeExposureNotificationsEnabled(!buds.volumeExposureNotificationsEnabled)
            }
          }

          Text {
            visible: root.guidanceVisible
            width: parent.width
            text: buds.daemonReachable
              ? "Connect your Pixel Buds Pro to see battery and ANC controls."
              : "Start the pixelbudsd daemon to see battery and ANC controls."
            color: root.dim
            font.family: root.fontFamily
            font.pixelSize: Style.font.body
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
          }
        }
      }
    }
  }

  Timer {
    id: phraseTimer
    interval: root.phraseIntervalMs
    running: root.opened && buds.hasBuds
    repeat: true
    onTriggered: phraseSwap.restart()
  }

  SequentialAnimation {
    id: phraseSwap
    PropertyAnimation {
      target: hero; property: "metaOpacity"
      to: 0.0; duration: 180; easing.type: Easing.OutQuad
    }
    ScriptAction {
      script: root.phraseIndex = (root.phraseIndex + 1) % root.activePhrases.length
    }
    PropertyAnimation {
      target: hero; property: "metaOpacity"
      to: 1.0; duration: 260; easing.type: Easing.InQuad
    }
  }

  component BudRow: Item {
    id: budRow
    property string label: ""
    property var bud: Model.defaultBud()

    readonly property string metaText: Model.budMeta(bud)
    readonly property bool low: bud.level !== Model.LEVEL_UNKNOWN
      && bud.level <= root.lowBatteryPercent && !bud.charging

    implicitHeight: budLayout.implicitHeight

    RowLayout {
      id: budLayout
      anchors.left: parent.left
      anchors.right: parent.right
      spacing: Style.space(8)

      Text {
        text: budRow.label
        color: root.foreground
        opacity: 0.6
        font.family: root.fontFamily
        font.pixelSize: Style.font.bodySmall
        Layout.preferredWidth: Style.space(44)
      }

      Rectangle {
        id: meterTrack
        Layout.fillWidth: true
        Layout.alignment: Qt.AlignVCenter
        implicitHeight: Style.space(6)
        radius: height / 2
        color: Qt.darker(root.foreground, 3.2)

        Rectangle {
          width: meterTrack.width * Model.levelFraction(budRow.bud.level)
          height: parent.height
          radius: parent.radius
          color: budRow.low ? root.urgent : root.foreground
        }
      }

      Text {
        text: Model.levelText(budRow.bud.level)
        color: root.foreground
        font.family: root.fontFamily
        font.pixelSize: Style.font.bodySmall
        horizontalAlignment: Text.AlignRight
        Layout.preferredWidth: Style.space(38)
      }

      Text {
        text: budRow.metaText
        color: root.dim
        font.family: root.fontFamily
        font.pixelSize: Style.font.caption
        elide: Text.ElideRight
        Layout.preferredWidth: Style.space(56)
      }
    }
  }

  component ModeRow: CursorSurface {
    id: modeRow
    property int mode: 0

    readonly property string rowName: "mode:" + mode
    readonly property bool selected: buds.ancMode === mode

    hasCursor: root.rowHasCursor(rowName)
    foreground: root.foreground
    implicitHeight: modeLabel.implicitHeight + Style.spacing.rowPaddingX

    MouseArea {
      anchors.fill: parent
      hoverEnabled: true
      cursorShape: Qt.PointingHandCursor
      onEntered: root.focusRow(modeRow.rowName)
      onClicked: buds.setAncMode(modeRow.mode)
    }

    RowLayout {
      anchors.left: parent.left
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
      anchors.leftMargin: Style.space(10)
      anchors.rightMargin: Style.space(10)
      spacing: Style.space(8)

      Text {
        id: modeLabel
        Layout.fillWidth: true
        text: Model.ancModeName(modeRow.mode)
        color: root.foreground
        opacity: modeRow.selected ? 1.0 : 0.75
        font.family: root.fontFamily
        font.pixelSize: Style.font.body
        elide: Text.ElideRight
      }

      Text {
        Layout.alignment: Qt.AlignVCenter
        text: Model.GLYPH_CHECK
        color: root.foreground
        font.family: root.fontFamily
        font.pixelSize: Style.font.icon
        opacity: modeRow.selected ? 1.0 : 0.0
      }
    }
  }

  component ToggleRow: CursorSurface {
    id: toggleRow
    property string rowName: ""
    property string label: ""
    property string caption: ""
    property bool checked: false

    signal toggled()

    hasCursor: root.rowHasCursor(rowName)
    foreground: root.foreground
    implicitHeight: toggleContent.implicitHeight + Style.spacing.rowPaddingX

    MouseArea {
      anchors.fill: parent
      hoverEnabled: true
      cursorShape: Qt.PointingHandCursor
      onEntered: root.focusRow(toggleRow.rowName)
      onClicked: toggleRow.toggled()
    }

    RowLayout {
      anchors.left: parent.left
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
      anchors.leftMargin: Style.space(10)
      anchors.rightMargin: Style.space(10)
      spacing: Style.space(8)

      ColumnLayout {
        id: toggleContent
        Layout.fillWidth: true
        spacing: Style.space(1)

        Text {
          Layout.fillWidth: true
          text: toggleRow.label
          color: root.foreground
          font.family: root.fontFamily
          font.pixelSize: Style.font.body
          elide: Text.ElideRight
        }

        Text {
          Layout.fillWidth: true
          text: toggleRow.caption
          color: root.dim
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
          elide: Text.ElideRight
        }
      }

      ToggleSwitch {
        Layout.alignment: Qt.AlignVCenter
        checked: toggleRow.checked
        busy: buds.busy
        hasCursor: toggleRow.hasCursor
        foreground: root.foreground
        onToggled: toggleRow.toggled()
      }
    }
  }
}
