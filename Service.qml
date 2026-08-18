import QtQuick
import Quickshell
import Quickshell.Io
import "Model.js" as Model

Item {
  id: root

  property var settings: ({})

  property bool daemonReachable: false
  property bool connected: false
  property string deviceName: ""
  property string modelName: ""
  property int ancMode: Model.ANC_UNKNOWN
  property bool multipointEnabled: false
  property bool onHeadDetectionEnabled: false
  property bool speechDetectionEnabled: false
  property bool volumeExposureNotificationsEnabled: false
  property bool schemaUnsupported: false
  property var leftBud: Model.defaultBud()
  property var rightBud: Model.defaultBud()
  property var caseBattery: Model.defaultCase()
  property string lastError: ""
  property string actionStatus: ""

  readonly property string ctlPath: String(setting("ctlPath", "") || "pixelbudsctl")
  readonly property bool busy: commandProcess.running
  // pixelbudsd publishes here on change, so there is nothing to poll.
  readonly property string statePath: (Quickshell.env("XDG_STATE_HOME")
    || Quickshell.env("HOME") + "/.local/state") + "/pixelbudspro/status.json"
  readonly property bool hasBuds: daemonReachable && connected
  // Unlike AirPods, the Pixel Buds Pro battery only arrives over the Maestro
  // RFCOMM session, which is only open while connected, so this does not need
  // a separate "connected" gate the way omapods' hasBattery does.
  readonly property bool hasBattery: hasBuds
    && (leftBud.level !== Model.LEVEL_UNKNOWN
      || rightBud.level !== Model.LEVEL_UNKNOWN
      || caseBattery.level !== Model.LEVEL_UNKNOWN)

  // How long an optimistic value is held before the daemon's own state wins.
  readonly property int settleHoldMs: 4000
  readonly property int actionStatusMs: 2200

  // Held over incoming reads until the daemon agrees, so a write already in flight
  // when the click landed cannot snap the control back.
  property string _pendingField: ""
  property var _pendingValue: null

  // Single slot: a verb sent while another is in flight replaces the queued one
  // rather than being dropped, which is what arrow-key repeat produces.
  property var _queued: null

  function setting(name, fallback) {
    var value = settings ? settings[name] : undefined
    return value === undefined || value === null ? fallback : value
  }

  function refresh() {
    stateFile.reload()
  }

  function applyLine(raw) {
    var status = Model.parseStatus(raw)
    if (!status.ok) {
      // A line we cannot read still proves the daemon is running and writing.
      daemonReachable = true
      connected = false
      schemaUnsupported = status.schemaTooNew
      lastError = status.lastError
      return
    }
    daemonReachable = true
    schemaUnsupported = false
    lastError = ""
    applyStatus(status)
  }

  // pixelbudsd removes the file when it stops, so an absent file is a stopped daemon.
  function stateGone() {
    daemonReachable = false
    connected = false
    schemaUnsupported = false
    lastError = ""
  }

  function applyStatus(status) {
    connected = status.connected
    deviceName = status.deviceName
    modelName = status.modelName
    leftBud = status.left
    rightBud = status.right
    caseBattery = status.caseBattery

    ancMode = _settle("ancMode", status.ancMode)
    multipointEnabled = _settle("multipointEnabled", status.multipointEnabled)
    onHeadDetectionEnabled = _settle("onHeadDetectionEnabled", status.onHeadDetectionEnabled)
    speechDetectionEnabled = _settle("speechDetectionEnabled", status.speechDetectionEnabled)
    volumeExposureNotificationsEnabled = _settle("volumeExposureNotificationsEnabled", status.volumeExposureNotificationsEnabled)
  }

  function _settle(field, reported) {
    if (_pendingField !== field) return reported
    if (reported === _pendingValue) {
      _clearPending()
      return reported
    }
    return _pendingValue
  }

  function _clearPending() {
    _pendingField = ""
    _pendingValue = null
    settleTimer.stop()
  }

  function _send(verb, field, optimistic) {
    if (verb === "") return
    if (commandProcess.running) {
      _queued = { verb: verb, field: field, optimistic: optimistic }
      _pendingField = field
      _pendingValue = optimistic
      root[field] = optimistic
      settleTimer.restart()
      return
    }
    _pendingField = field
    _pendingValue = optimistic
    root[field] = optimistic
    settleTimer.restart()
    commandProcess.command = [ctlPath, verb]
    commandProcess.running = true
  }

  function setAncMode(mode) {
    _send(Model.ancModeVerb(mode), "ancMode", mode)
  }

  function cycleAncMode() {
    if (!hasBuds) return
    var modes = Model.availableModes()
    var at = modes.indexOf(ancMode)
    // An unknown current mode has no next one, so start at the head instead of past it.
    setAncMode(at < 0 ? modes[0] : modes[(at + 1) % modes.length])
  }

  function setMultipointEnabled(enabled) {
    _send(enabled ? "multipoint:on" : "multipoint:off", "multipointEnabled", enabled)
  }

  function setOnHeadDetectionEnabled(enabled) {
    _send(enabled ? "ohd:on" : "ohd:off", "onHeadDetectionEnabled", enabled)
  }

  function setSpeechDetectionEnabled(enabled) {
    _send(enabled ? "speech:on" : "speech:off", "speechDetectionEnabled", enabled)
  }

  function setVolumeExposureNotificationsEnabled(enabled) {
    _send(enabled ? "volumeexposure:on" : "volumeexposure:off", "volumeExposureNotificationsEnabled", enabled)
  }

  Timer {
    // Bounds the optimistic hold, and re-reads because a verb that changed nothing
    // leaves the daemon's file untouched, so no watch fires to correct the display.
    id: settleTimer
    interval: root.settleHoldMs
    repeat: false
    onTriggered: { root._clearPending(); root.refresh() }
  }

  Timer {
    id: actionStatusTimer
    interval: root.actionStatusMs
    repeat: false
    onTriggered: root.actionStatus = ""
  }

  FileView {
    id: stateFile
    path: root.statePath
    watchChanges: true
    printErrors: false
    // text() is stale inside the change signal, so both paths go through reload.
    onFileChanged: reload()
    onLoaded: root.applyLine(text())
    onLoadFailed: root.stateGone()
  }

  Process {
    id: commandProcess
    running: false
    command: []
    stderr: StdioCollector { id: commandErr; waitForEnd: true }
    onExited: function (exitCode) {
      if (exitCode !== 0) {
        // Clearing the hold also stops the timer that would have re-read, so do it here.
        root._clearPending()
        root.refresh()
        root._queued = null
        // Its own field with its own timer, or the next status read wipes it unread.
        root.actionStatus = Model.elideError(commandErr.text || "pixelbudsctl rejected the command")
        actionStatusTimer.restart()
      }
      if (root._queued) {
        var next = root._queued
        root._queued = null
        root._send(next.verb, next.field, next.optimistic)
      }
    }
  }
}
