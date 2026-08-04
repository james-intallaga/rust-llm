import Foundation
import Speech
import Combine
import AVFoundation

@MainActor
class SpeechRecognizer: ObservableObject {
    @Published var transcript: String = ""
    @Published var isRecording: Bool = false

    private var audioEngine: AVAudioEngine?
    private var request: SFSpeechAudioBufferRecognitionRequest?
    private var task: SFSpeechRecognitionTask?
    private let recognizer: SFSpeechRecognizer?

    init() {
        self.recognizer = SFSpeechRecognizer()
    }

    func start() {
        SFSpeechRecognizer.requestAuthorization { status in
            guard status == .authorized else { return }

            Task { @MainActor in
                self.startRecording()
            }
        }
    }

    func stop() {
        task?.finish()
        task = nil
        request = nil
        audioEngine?.stop()
        audioEngine?.inputNode.removeTap(onBus: 0)
        isRecording = false
    }

    private func startRecording() {
        do {
            audioEngine = AVAudioEngine()
            request = SFSpeechAudioBufferRecognitionRequest()

            guard let audioEngine = audioEngine, let request = request else { return }

            let recordingSession = AVAudioSession.sharedInstance()
            try recordingSession.setCategory(.record, mode: .measurement, options: .duckOthers)
            try recordingSession.setActive(true, options: .notifyOthersOnDeactivation)

            let inputNode = audioEngine.inputNode
            let recordingFormat = inputNode.outputFormat(forBus: 0)

            inputNode.installTap(onBus: 0, bufferSize: 1024, format: recordingFormat) { buffer, _ in
                request.append(buffer)
            }

            audioEngine.prepare()
            try audioEngine.start()

            isRecording = true
            transcript = ""

            task = recognizer?.recognitionTask(with: request) { result, error in
                if let result = result {
                    self.transcript = result.bestTranscription.formattedString
                }
                if error != nil || result?.isFinal == true {
                    self.stop()
                }
            }
        } catch {
            print("Speech recognition failed: \(error)")
            stop()
        }
    }
}
