```mermaid
flowchart TB
    subgraph Input
        filesrc["<b>filesrc</b>\n(Read input file)"]
        decodebin["<b>decodebin</b>\n(Decode media)"]
    end

    subgraph Video_Processing
        direction TB
        videoconvert1["<b>videoconvert1</b>\n(Convert format)"]
        videoscale1["<b>videoscale1</b>\n(Scale video)"]
        queue1["<b>queue1</b>\n(Buffer frames)"]
        appsink["<b>appsink</b>\n(Receive frames)"]
        
        callback["<b>User Frame Callback</b>\n(Process each frame)"]
        
        appsrc["<b>appsrc</b>\n(Send processed frames)"]
        queue2["<b>queue2</b>\n(Buffer frames)"]
        videoconvert2["<b>videoconvert2</b>\n(Convert format)"]
        videoscale2["<b>videoscale2</b>\n(Scale video)"]
        x264enc["<b>x264enc</b>\n(Encode H.264)"]
        h264parse["<b>h264parse</b>\n(Parse H.264)"]
        queue3["<b>queue3</b>\n(Buffer frames)"]
    end

    subgraph Audio_Processing
        direction TB
        queue_audio["<b>queue_audio</b>\n(Buffer audio)"]
        audioconvert["<b>audioconvert</b>\n(Convert format)"]
        audioresample["<b>audioresample</b>\n(Resample audio)"]
        aacenc["<b>aacenc</b>\n(Encode AAC)"]
        aacparse["<b>aacparse</b>\n(Parse AAC)"]
    end

    subgraph Output
        muxer["<b>mp4mux</b>\n(Combine streams)"]
        filesink["<b>filesink</b>\n(Write output file)"]
    end

    %% Connections
    filesrc --> decodebin
    
    %% Video path
    decodebin -->|"video"| videoconvert1
    videoconvert1 --> videoscale1
    videoscale1 --> queue1
    queue1 --> appsink
    appsink -->|"RGBA frames"| callback
    callback -->|"Processed frames"| appsrc
    appsrc --> queue2
    queue2 --> videoconvert2
    videoconvert2 --> videoscale2
    videoscale2 --> x264enc
    x264enc --> h264parse
    h264parse --> queue3
    queue3 --> muxer
    
    %% Audio path
    decodebin -->|"audio"| queue_audio
    queue_audio --> audioconvert
    audioconvert --> audioresample
    audioresample --> aacenc
    aacenc --> aacparse
    aacparse --> muxer
    
    %% Output
    muxer --> filesink

    %% Styles
    classDef default fill:#f9f9f9,stroke:#333,stroke-width:2px;
    classDef input fill:#d5e8d4,stroke:#82b366,color:#000;
    classDef process fill:#dae8fc,stroke:#6c8ebf,color:#000;
    classDef output fill:#ffe6cc,stroke:#d79b00,color:#000;
    classDef audio fill:#e1d5e7,stroke:#9673a6,color:#000;
    
    class filesrc,decodebin input;
    class videoconvert1,videoscale1,queue1,appsink,callback,appsrc,queue2,videoconvert2,videoscale2,x264enc,h264parse,queue3 process;
    class queue_audio,audioconvert,audioresample,aacenc,aacparse audio;
    class muxer,filesink output;
```